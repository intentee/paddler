use paddler_cluster_harness::agent_config::AgentConfig;
use paddler_cluster_harness::agent_desired_model_at::agent_desired_model_at;
use paddler_cluster_harness::cluster::Cluster;
use paddler_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_cluster_harness::cluster_params::ClusterParams;
use paddler_file_fixture::fixture_path::fixture_path;
use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;

#[tokio::test(flavor = "multi_thread")]
async fn loading_an_invalid_gguf_model_reports_that_it_cannot_be_loaded() {
    let model_path_on_agent = fixture_path("invalid.gguf").display().to_string();
    let mut cluster = Cluster::start_in_process(ClusterParams {
        agents: AgentConfig::uniform(1, 1),
        desired_state: ClusterDesiredState::Apply(Box::new(BalancerDesiredState {
            model: agent_desired_model_at(&fixture_path("invalid.gguf"))
                .expect("the invalid model fixture path must be absolute"),
            ..BalancerDesiredState::default()
        })),
        wait_for_slots_ready: false,
        ..ClusterParams::default()
    })
    .await
    .expect("a single-agent cluster must start");

    cluster
        .wait_for_first_agent_issue(|issue| {
            matches!(issue, AgentIssue::ModelCannotBeLoaded(model_path) if model_path.model_path == model_path_on_agent)
        })
        .await
        .expect("the agent must report ModelCannotBeLoaded for the configured path");

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
