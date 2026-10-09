use tokio_util::sync::CancellationToken;

use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_model_source::model_source::ModelSource;
use paddler_test_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::model_card::ModelCard;
use paddler_test_cluster_harness::model_card::smolvlm2_256m::smolvlm2_256m;
use paddler_tests::desired_state_with_multimodal_projection::desired_state_with_multimodal_projection;
use paddler_tests::start_cluster::start_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_persists_local_mmproj_path_in_desired_state() {
    let ModelCard { reference } = smolvlm2_256m();

    let local_mmproj_path = "/tmp/test-mmproj.gguf".to_owned();

    let desired_state = desired_state_with_multimodal_projection(
        BalancerDesiredState {
            model: ModelSource::HuggingFace(reference).into_agent_desired_model(),
            ..BalancerDesiredState::default()
        },
        ModelSource::LocalToAgent(local_mmproj_path).into_agent_desired_model(),
    );

    let cluster = start_cluster(ClusterParams {
        agents: Vec::new(),
        wait_for_slots_ready: false,
        desired_state: ClusterDesiredState::Apply(Box::new(desired_state.clone())),
        ..ClusterParams::default()
    })
    .await
    .expect("the cluster must start");

    let retrieved = cluster
        .client_management
        .get_balancer_desired_state(CancellationToken::new())
        .await
        .expect("failed to read balancer desired state");

    assert_eq!(retrieved, desired_state);

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
