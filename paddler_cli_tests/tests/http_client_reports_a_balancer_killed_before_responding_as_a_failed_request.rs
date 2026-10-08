use serde_json::json;
use tokio::spawn;
use tokio_util::sync::CancellationToken;

use paddler_balancer::compatibility::openai_service::openai_api_path::OpenAIApiPath;
use paddler_cli_tests::start_signalled_subprocess_cluster::start_signalled_subprocess_cluster;
use paddler_cli_tests::subprocess_cluster::SubprocessCluster;
use paddler_client::error::Error;
use paddler_client::http_client::HttpClient;
use paddler_test_cluster_harness::cluster_params::ClusterParams;

#[tokio::test(flavor = "multi_thread")]
async fn http_client_reports_a_balancer_killed_before_responding_as_a_failed_request() {
    let SubprocessCluster {
        balancer_signals,
        mut cluster,
    } = start_signalled_subprocess_cluster(
        env!("CARGO_BIN_EXE_paddler_cluster_node"),
        ClusterParams {
            agents: Vec::new(),
            wait_for_slots_ready: false,
            ..ClusterParams::default()
        },
    )
    .await
    .expect("a balancer without agents must start");
    let http_client = HttpClient::new(
        cluster
            .balancer
            .compat_openai_base_url()
            .expect("the balancer must serve the OpenAI compatibility service"),
    );
    let chat_completion = spawn(async move {
        http_client
            .post_json(
                CancellationToken::new(),
                OpenAIApiPath::CHAT_COMPLETIONS,
                &json!({
                    "model": "test-model",
                    "messages": [{"role": "user", "content": "hi"}]
                }),
            )
            .await
    });

    cluster
        .wait_for_buffered_request_count(1)
        .await
        .expect("the chat completion must wait in the buffer while no agent is available");
    balancer_signals
        .kill()
        .expect("the balancer must be killed before it responds");

    assert!(matches!(
        chat_completion.await.expect("the chat completion task must not panic"),
        Err(Error::Http(source)) if source.is_request()
    ));

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
