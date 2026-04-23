#![cfg(all(
    feature = "tests_that_use_compiled_paddler",
    feature = "tests_that_use_llms"
))]

use futures_util::StreamExt;
use paddler_integration_tests::managed_cluster::ManagedCluster;
use paddler_integration_tests::managed_cluster_params::ManagedClusterParams;
use paddler_types::generated_token_result::GeneratedTokenResult;
use paddler_types::inference_client::Message;
use paddler_types::inference_client::Response;
use paddler_types::request_params::ContinueFromRawPromptParams;
use serial_test::file_serial;

#[tokio::test]
#[file_serial]
async fn test_agent_exits_gracefully_during_active_generation() -> anyhow::Result<()> {
    let mut cluster = ManagedCluster::spawn(ManagedClusterParams {
        agent_slots: 1,
        ..ManagedClusterParams::default()
    })
    .await?;

    let mut stream = cluster
        .balancer
        .client()
        .inference()
        .continue_from_raw_prompt(ContinueFromRawPromptParams {
            grammar: None,
            max_tokens: 1000,
            raw_prompt: "Write a very long essay about the history of computing".to_owned(),
        })
        .await?;

    // Wait for first token to confirm generation is active
    let first_message = stream.next().await;
    assert!(
        first_message.is_some(),
        "Should receive at least one message from inference"
    );

    if let Some(Ok(Message::Response(envelope))) = &first_message {
        assert!(
            matches!(
                envelope.response,
                Response::GeneratedToken(GeneratedTokenResult::Token(_))
            ),
            "First message should be a token"
        );
    }

    // Kill the agent (SIGTERM) while generation is active
    cluster.agent.kill();

    // Verify balancer detects agent removal
    cluster.balancer.wait_for_agent_count(0).await?;

    Ok(())
}
