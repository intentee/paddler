use tokio_util::sync::CancellationToken;

use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::inference_mode::InferenceMode;
use paddler_test_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::model_card::ModelCard;
use paddler_test_cluster_harness::model_card::smolvlm2_256m::smolvlm2_256m;
use paddler_test_cluster_harness::model_card::smolvlm2_256m_mmproj::smolvlm2_256m_mmproj;
use paddler_tests::desired_state_with_multimodal_projection::desired_state_with_multimodal_projection;
use paddler_tests::start_cluster::start_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_persists_huggingface_mmproj_in_desired_state() {
    let ModelCard {
        reference: primary_reference,
    } = smolvlm2_256m();
    let ModelCard {
        reference: mmproj_reference,
    } = smolvlm2_256m_mmproj();

    let desired_state = desired_state_with_multimodal_projection(
        BalancerDesiredState {
            model: AgentDesiredModel::HuggingFace(primary_reference),
            ..BalancerDesiredState::unconfigured(InferenceMode::TextGeneration)
        },
        AgentDesiredModel::HuggingFace(mmproj_reference),
    )
    .expect("a text generation state must accept a multimodal projection");

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
