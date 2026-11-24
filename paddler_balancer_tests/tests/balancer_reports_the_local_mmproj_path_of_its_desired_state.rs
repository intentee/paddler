use std::path::Path;

use tokio_util::sync::CancellationToken;

use paddler_balancer_desired_state_fixture::desired_state_with_multimodal_projection::desired_state_with_multimodal_projection;
use paddler_cluster_harness::agent_desired_model_at::agent_desired_model_at;
use paddler_cluster_harness::cluster::Cluster;
use paddler_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_cluster_harness::cluster_params::ClusterParams;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_model_card_fixture::model_card::ModelCard;
use paddler_model_card_fixture::smolvlm2_256m::smolvlm2_256m;
use paddler_model_source::model_source::ModelSource;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_reports_the_local_mmproj_path_of_its_desired_state() {
    let ModelCard { reference } = smolvlm2_256m();

    let local_mmproj_path = "/tmp/test-mmproj.gguf".to_owned();

    let desired_state = desired_state_with_multimodal_projection(
        BalancerDesiredState {
            model: ModelSource::HuggingFace(reference).into_agent_desired_model(),
            ..BalancerDesiredState::default()
        },
        agent_desired_model_at(Path::new(&local_mmproj_path))
            .expect("the multimodal projection path must be absolute"),
    );

    let cluster = Cluster::start_in_process(ClusterParams {
        desired_state: ClusterDesiredState::Apply(Box::new(desired_state.clone())),
        ..ClusterParams::without_agents()
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
