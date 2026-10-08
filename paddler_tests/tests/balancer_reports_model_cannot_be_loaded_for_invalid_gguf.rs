use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::decision_settings::DecisionSettings;
use paddler_messaging::inference_mode::InferenceMode;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::start_cluster::start_cluster;

const DECISION_SLOTS: u16 = 2;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_reports_model_cannot_be_loaded_for_invalid_gguf() {
    let model_path_on_agent =
        concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/invalid.gguf").to_owned();
    let pointer_head_path_on_agent = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../fixtures/qwen3_5_0_8b_synthetic_pointer_head.gguf"
    )
    .to_owned();

    for inference_mode in [
        InferenceMode::Decision,
        InferenceMode::Embeddings,
        InferenceMode::TextGeneration,
    ] {
        let mut cluster = start_cluster(ClusterParams {
            agents: AgentConfig::uniform(1, DECISION_SLOTS),
            desired_state: ClusterDesiredState::Apply(Box::new(BalancerDesiredState {
                decision: DecisionSettings {
                    pointer_head: AgentDesiredModel::LocalToAgent(
                        pointer_head_path_on_agent.clone(),
                    ),
                },
                inference_mode,
                model: AgentDesiredModel::LocalToAgent(model_path_on_agent.clone()),
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
}
