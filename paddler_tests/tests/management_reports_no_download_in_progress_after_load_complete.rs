#![cfg(feature = "tests_that_use_llms")]

use tokio_util::sync::CancellationToken;

use paddler_messaging::model_download_status::ModelDownloadStatus;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;

#[tokio::test(flavor = "multi_thread")]
async fn management_reports_no_download_in_progress_after_load_complete() {
    let cluster = start_cluster_with_qwen3(AgentConfig::uniform(1, 2))
        .await
        .expect("the cluster must start");

    let snapshot = cluster
        .client_management
        .get_agents(CancellationToken::new())
        .await
        .expect("get_agents should succeed");

    assert_eq!(
        snapshot
            .agents
            .iter()
            .map(|agent| &agent.status.download_status)
            .collect::<Vec<_>>(),
        vec![&ModelDownloadStatus::NotDownloading]
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
