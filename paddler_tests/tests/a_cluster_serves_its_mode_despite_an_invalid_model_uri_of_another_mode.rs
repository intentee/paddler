#![cfg(feature = "tests_that_use_llms")]

use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::decision_settings::DecisionSettings;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::model_card::qwen3_0_6b::qwen3_0_6b;
use paddler_tests::inference_socket_round_trip::inference_socket_round_trip;
use paddler_tests::start_cluster::start_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn a_cluster_serves_its_mode_despite_an_invalid_model_uri_of_another_mode() {
    let cluster = start_cluster(ClusterParams {
        agents: AgentConfig::uniform(1, 1),
        desired_state: ClusterDesiredState::Apply(Box::new(BalancerDesiredState {
            decision: DecisionSettings {
                pointer_head: AgentDesiredModel::Uri("not a valid uri".to_owned()),
            },
            ..qwen3_0_6b().into_desired_state()
        })),
        wait_for_slots_ready: true,
        ..ClusterParams::default()
    })
    .await
    .expect("text generation must start without reporting the decision pointer head URI");

    inference_socket_round_trip(&cluster.client_inference)
        .await
        .expect("the cluster must generate tokens");

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
