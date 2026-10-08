use anyhow::Context as _;
use anyhow::Error;
use anyhow::Result;
use tokio_util::sync::CancellationToken;

use paddler_client::client_management::ClientManagement;
use paddler_inference_parameters::sampling_parameters::SamplingParameters;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::balancer_text_generation_settings::BalancerTextGenerationSettings;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::start_cluster::start_cluster;

async fn get_balancer_desired_state(
    client_management: &ClientManagement,
) -> Result<BalancerDesiredState> {
    client_management
        .get_balancer_desired_state(CancellationToken::new())
        .await
        .map_err(Error::new)
        .context("the balancer desired state must be served")
}

#[tokio::test(flavor = "multi_thread")]
async fn management_returns_balancer_desired_state_that_was_put() {
    let cluster = start_cluster(ClusterParams {
        agents: Vec::new(),
        wait_for_slots_ready: false,
        ..ClusterParams::default()
    })
    .await
    .expect("the cluster must start");

    assert_eq!(
        get_balancer_desired_state(&cluster.client_management)
            .await
            .expect("the balancer must report its desired state"),
        BalancerDesiredState::default()
    );

    let desired_state = BalancerDesiredState {
        text_generation: BalancerTextGenerationSettings {
            sampling_parameters: SamplingParameters::deterministic(),
            ..BalancerTextGenerationSettings::default()
        },
        ..BalancerDesiredState::default()
    };

    cluster
        .client_management
        .put_balancer_desired_state(CancellationToken::new(), &desired_state)
        .await
        .expect("the balancer must accept the desired state");

    assert_eq!(
        get_balancer_desired_state(&cluster.client_management)
            .await
            .expect("the balancer must report its desired state"),
        desired_state
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
