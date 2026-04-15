#![cfg(all(
    feature = "tests_that_use_compiled_paddler",
    feature = "tests_that_use_llms"
))]

use std::pin::Pin;

use futures_util::Stream;
use futures_util::StreamExt;
use paddler_client::Result as ClientResult;
use paddler_integration_tests::managed_cluster::ManagedCluster;
use paddler_integration_tests::managed_cluster_params::ManagedClusterParams;
use paddler_types::generated_token_result::GeneratedTokenResult;
use paddler_types::inference_client::Message;
use paddler_types::inference_client::Response;
use paddler_types::request_params::ContinueFromRawPromptParams;
use serial_test::file_serial;

async fn collect_text_from_stream(
    mut stream: Pin<Box<dyn Stream<Item = ClientResult<Message>> + Send>>,
) -> String {
    let mut text = String::new();

    while let Some(message) = stream.next().await {
        let message = message.expect("message should deserialize");

        match message {
            Message::Response(envelope) => match envelope.response {
                Response::GeneratedToken(GeneratedTokenResult::Token(token)) => {
                    text.push_str(&token);
                }
                Response::GeneratedToken(GeneratedTokenResult::Done) => break,
                other => panic!("unexpected response: {other:?}"),
            },
            Message::Error(envelope) => {
                panic!(
                    "unexpected error: {} - {}",
                    envelope.error.code, envelope.error.description
                );
            }
        }
    }

    text
}

#[tokio::test]
#[file_serial]
async fn test_concurrent_generation_requests_from_multiple_clients() {
    let cluster = ManagedCluster::spawn(ManagedClusterParams {
        agent_name: "concurrent-generation-agent".to_string(),
        agent_slots: 4,
        ..ManagedClusterParams::default()
    })
    .await
    .expect("failed to spawn cluster");

    let prompts = [
        "The capital of France is",
        "Two plus two equals",
        "Water freezes at",
        "The sun rises in the",
    ];

    let client_tasks = prompts.iter().map(|prompt| {
        let client = cluster.balancer.client();
        let prompt_string = (*prompt).to_string();

        async move {
            let stream = client
                .inference()
                .continue_from_raw_prompt(ContinueFromRawPromptParams {
                    grammar: None,
                    max_tokens: 10,
                    raw_prompt: prompt_string,
                })
                .await
                .expect("raw prompt request should succeed");

            collect_text_from_stream(stream).await
        }
    });

    let per_client_results = futures_util::future::join_all(client_tasks).await;

    assert_eq!(per_client_results.len(), prompts.len());

    for (prompt_index, generated_text) in per_client_results.iter().enumerate() {
        assert!(
            !generated_text.is_empty(),
            "client {prompt_index} should receive at least one token"
        );
    }
}
