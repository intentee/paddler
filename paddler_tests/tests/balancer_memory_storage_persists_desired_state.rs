use tokio_util::sync::CancellationToken;

use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_test_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::model_card::ModelCard;
use paddler_test_cluster_harness::model_card::qwen3_0_6b::qwen3_0_6b;
use paddler_tests::start_cluster::start_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_memory_storage_persists_desired_state() {
    let ModelCard { reference } = qwen3_0_6b();

    let desired_state = BalancerDesiredState {
        model: AgentDesiredModel::HuggingFace(reference),
        ..BalancerDesiredState::default()
    };

    let cluster = start_cluster(ClusterParams {
        agents: Vec::new(),
        wait_for_slots_ready: false,
        state_database_url: "memory://".to_owned(),
        desired_state: ClusterDesiredState::Apply(Box::new(desired_state.clone())),
        ..ClusterParams::default()
    })
    .await
    .expect("the cluster must start");

    let observed = cluster
        .client_management
        .get_balancer_desired_state(CancellationToken::new())
        .await
        .expect("failed to read desired state from memory backend");

    assert_eq!(observed.model, desired_state.model);

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
