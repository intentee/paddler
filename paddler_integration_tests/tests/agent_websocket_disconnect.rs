#![cfg(all(
    feature = "tests_that_use_compiled_paddler",
    feature = "tests_that_use_llms"
))]

use std::time::Duration;
use std::time::Instant;

use futures_util::StreamExt;
use paddler_client::PaddlerClient;
use paddler_integration_tests::BALANCER_INFERENCE_ADDR;
use paddler_integration_tests::BALANCER_MANAGEMENT_ADDR;
use paddler_integration_tests::managed_cluster::ManagedCluster;
use paddler_integration_tests::managed_cluster_params::ManagedClusterParams;
use paddler_types::generated_token_result::GeneratedTokenResult;
use paddler_types::inference_client::Message;
use paddler_types::inference_client::Response;
use paddler_types::request_params::ContinueFromRawPromptParams;
use serial_test::file_serial;
use url::Url;

#[tokio::test]
#[file_serial]
async fn test_slot_released_after_websocket_disconnect() {
    let cluster = ManagedCluster::spawn(ManagedClusterParams {
        agent_slots: 1,
        ..ManagedClusterParams::default()
    })
    .await
    .expect("failed to spawn cluster");

    // Create a SEPARATE client (not the cluster's pooled client).
    // When we drop this client, the WebSocket TCP connection closes —
    // simulating what happens when a browser tab is closed.
    let disposable_client = PaddlerClient::new(
        Url::parse(&format!("http://{BALANCER_INFERENCE_ADDR}"))
            .expect("valid inference url"),
        Url::parse(&format!("http://{BALANCER_MANAGEMENT_ADDR}"))
            .expect("valid management url"),
        1,
    );

    let mut stream = disposable_client
        .inference()
        .continue_from_raw_prompt(ContinueFromRawPromptParams {
            grammar: None,
            max_tokens: 500,
            raw_prompt: "Write a very long essay about the history of philosophy".to_string(),
        })
        .await
        .expect("inference request should connect");

    // Wait for first token — confirms generation is active
    let first_message = stream.next().await;
    assert!(first_message.is_some(), "Should receive at least one message");

    if let Some(Ok(Message::Response(envelope))) = &first_message {
        assert!(
            matches!(
                envelope.response,
                Response::GeneratedToken(GeneratedTokenResult::Token(_))
            ),
            "First message should be a token, got: {:?}",
            envelope.response
        );
    }

    // Verify slot is occupied
    cluster.balancer.wait_for_slots_processing(1).await;

    // CLOSE THE WEBSOCKET — drop stream AND client.
    // This closes the TCP connection, just like closing a browser tab.
    drop(stream);
    drop(disposable_client);

    // Slot must be released within 5 seconds.
    // Without the fix, it stays occupied until inference_item_timeout (30s default).
    let deadline = Instant::now() + paddler_integration_tests::WAIT_FOR_STATE_CHANGE_TIMEOUT;
    let disconnect_instant = Instant::now();
    let mut final_slots_processing = -1;

    loop {
        if let Ok(snapshot) = cluster
            .balancer
            .client()
            .management()
            .get_agents()
            .await
        {
            let total: i32 = snapshot
                .agents
                .iter()
                .map(|agent| agent.slots_processing)
                .sum();

            final_slots_processing = total;

            if total == 0 {
                break;
            }
        }

        assert!(
            Instant::now() < deadline,
            "Timed out waiting for slot to be released after WebSocket disconnect. \
             slots_processing is still {final_slots_processing}. \
             The stop signal from WebSocket close is not reaching the scheduler."
        );

        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}
