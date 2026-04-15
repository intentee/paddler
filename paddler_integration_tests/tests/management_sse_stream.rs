#![cfg(all(
    feature = "tests_that_use_compiled_paddler",
    feature = "tests_that_use_llms"
))]

use std::time::Duration;

use futures_util::StreamExt;
use paddler_integration_tests::managed_cluster::ManagedCluster;
use paddler_integration_tests::managed_cluster_params::ManagedClusterParams;
use paddler_types::request_params::ContinueFromRawPromptParams;
use serial_test::file_serial;

/// Simulates two browser tabs keeping SSE open. After initial snapshot,
/// trigger generation, then wait for both to receive the update.
/// The critical check: BOTH tabs get the second event, not just one.
#[tokio::test]
#[file_serial]
async fn test_both_tabs_receive_ongoing_sse_updates() -> anyhow::Result<()> {
    let cluster = ManagedCluster::spawn(ManagedClusterParams {
        agent_slots: 1,
        ..ManagedClusterParams::default()
    })
    .await?;

    let agents_url = format!(
        "http://{}/api/v1/agents/stream",
        cluster.balancer.management_addr()
    );
    let client = reqwest::Client::new();

    let mut tab1 = client.get(&agents_url).send().await?;
    let mut tab2 = client.get(&agents_url).send().await?;

    let _ = tokio::time::timeout(Duration::from_secs(5), tab1.chunk()).await??;
    let _ = tokio::time::timeout(Duration::from_secs(5), tab2.chunk()).await??;

    let mut inference = cluster
        .balancer
        .client()
        .inference()
        .continue_from_raw_prompt(ContinueFromRawPromptParams {
            grammar: None,
            max_tokens: 20,
            raw_prompt: "Count to ten".to_string(),
        })
        .await?;

    let _ = inference.next().await;

    let tab1_update = tokio::time::timeout(Duration::from_secs(5), tab1.chunk()).await??;
    let tab2_update = tokio::time::timeout(Duration::from_secs(5), tab2.chunk()).await??;

    let tab1_got_data = tab1_update.is_some();
    let tab2_got_data = tab2_update.is_some();

    eprintln!("tab1 update: {tab1_got_data}, tab2 update: {tab2_got_data}");

    assert!(tab1_got_data, "Tab 1 must receive SSE update");
    assert!(tab2_got_data, "Tab 2 must receive SSE update");

    while let Some(msg) = inference.next().await {
        if msg.is_err() {
            break;
        }
    }

    let tab1_final = tokio::time::timeout(Duration::from_secs(5), tab1.chunk()).await??;
    let tab2_final = tokio::time::timeout(Duration::from_secs(5), tab2.chunk()).await??;

    let tab1_final_ok = tab1_final.is_some();
    let tab2_final_ok = tab2_final.is_some();

    eprintln!("tab1 final: {tab1_final_ok}, tab2 final: {tab2_final_ok}");

    assert!(tab1_final_ok, "Tab 1 must receive final SSE update");
    assert!(tab2_final_ok, "Tab 2 must receive final SSE update");

    drop(cluster);

    Ok(())
}
