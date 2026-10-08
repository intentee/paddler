use anyhow::Result;

use paddler_inference_parameters::model_runtime_parameters::ModelRuntimeParameters;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_test_cluster_harness::cluster::Cluster;
use paddler_test_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::pointer_head_fixture::pointer_head_fixture;

use crate::decision_cluster_params::DecisionClusterParams;
use crate::start_cluster::start_cluster;

pub async fn start_decision_cluster(
    DecisionClusterParams {
        agents,
        context_size,
        model_card,
        n_batch,
        pointer_head_fixture: pointer_head_fixture_name,
        wait_for_slots_ready,
    }: DecisionClusterParams,
) -> Result<Cluster> {
    let desired_state =
        model_card.into_decision_desired_state(pointer_head_fixture(pointer_head_fixture_name));

    start_cluster(ClusterParams {
        agents,
        desired_state: ClusterDesiredState::Apply(Box::new(BalancerDesiredState {
            model_runtime_parameters: ModelRuntimeParameters {
                context_size,
                n_batch,
                ..desired_state.model_runtime_parameters
            },
            ..desired_state
        })),
        wait_for_slots_ready,
        ..ClusterParams::default()
    })
    .await
}
