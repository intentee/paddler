#![cfg(all(
    feature = "tests_that_use_compiled_paddler",
    feature = "tests_that_use_llms"
))]

use std::time::Duration;

use futures_util::StreamExt;
use paddler_integration_tests::BALANCER_MANAGEMENT_ADDR;
use paddler_integration_tests::managed_cluster::ManagedCluster;
use paddler_integration_tests::managed_cluster_params::ManagedClusterParams;
use paddler_types::request_params::ContinueFromRawPromptParams;
use serial_test::file_serial;

/// Simulates two browser tabs keeping SSE open. After initial snapshot,
/// trigger generation, then wait for both to receive the update.
/// The critical check: BOTH tabs get the second event, not just one.
#[tokio::test]
#[file_serial]
async fn test_both_tabs_receive_ongoing_sse_updates() {
    let cluster = ManagedCluster::spawn(ManagedClusterParams {
        agent_slots: 1,
        ..ManagedClusterParams::default()
    })
    .await
    .expect("failed to spawn cluster");

    let agents_url = format!("http://{BALANCER_MANAGEMENT_ADDR}/api/v1/agents/stream");
    let client = reqwest::Client::new();

    // Two tabs, each with a long-lived SSE connection
    let mut tab1 = client.get(&agents_url).send().await.expect("tab1 connect");
    let mut tab2 = client.get(&agents_url).send().await.expect("tab2 connect");

    // Both get initial snapshot
    let _ = tokio::time::timeout(Duration::from_secs(5), tab1.chunk())
        .await.expect("tab1 initial timeout").expect("tab1 err");
    let _ = tokio::time::timeout(Duration::from_secs(5), tab2.chunk())
        .await.expect("tab2 initial timeout").expect("tab2 err");

    // Now trigger a state change
    let mut inference = cluster
        .balancer
        .client()
        .inference()
        .continue_from_raw_prompt(ContinueFromRawPromptParams {
            grammar: None,
            max_tokens: 20,
            raw_prompt: "Count to ten".to_string(),
        })
        .await
        .expect("inference failed");

    // Wait for generation to start
    let _ = inference.next().await;

    // BOTH tabs must receive the update
    let tab1_update = tokio::time::timeout(Duration::from_secs(5), tab1.chunk()).await;
    let tab2_update = tokio::time::timeout(Duration::from_secs(5), tab2.chunk()).await;

    let tab1_got_data = tab1_update
        .expect("tab1 timed out waiting for update — SSE broken for second listener")
        .expect("tab1 update error")
        .is_some();
    let tab2_got_data = tab2_update
        .expect("tab2 timed out waiting for update — SSE broken for second listener")
        .expect("tab2 update error")
        .is_some();

    eprintln!("tab1 update: {tab1_got_data}, tab2 update: {tab2_got_data}");

    assert!(tab1_got_data, "Tab 1 must receive SSE update");
    assert!(tab2_got_data, "Tab 2 must receive SSE update");

    // Wait for generation to complete, then check BOTH get the completion update too
    while let Some(msg) = inference.next().await {
        if msg.is_err() {
            break;
        }
    }

    let tab1_final = tokio::time::timeout(Duration::from_secs(5), tab1.chunk()).await;
    let tab2_final = tokio::time::timeout(Duration::from_secs(5), tab2.chunk()).await;

    let tab1_final_ok = tab1_final
        .expect("tab1 timed out on final update")
        .expect("tab1 final error")
        .is_some();
    let tab2_final_ok = tab2_final
        .expect("tab2 timed out on final update")
        .expect("tab2 final error")
        .is_some();

    eprintln!("tab1 final: {tab1_final_ok}, tab2 final: {tab2_final_ok}");

    assert!(tab1_final_ok, "Tab 1 must receive final SSE update");
    assert!(tab2_final_ok, "Tab 2 must receive final SSE update");

    drop(cluster);
}
