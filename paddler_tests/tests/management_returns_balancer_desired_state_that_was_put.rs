use anyhow::Context as _;
use anyhow::Result;
use paddler_client::client_management::ClientManagement;
use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::inference_parameters::InferenceParameters;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::start_cluster::start_cluster;

async fn get_balancer_desired_state(
    client_management: &ClientManagement,
) -> Result<BalancerDesiredState> {
    client_management
        .get_balancer_desired_state()
        .await
        .map_err(anyhow::Error::new)
        .context("failed to GET /api/v1/balancer_desired_state")
}

#[tokio::test(flavor = "multi_thread")]
async fn management_returns_balancer_desired_state_that_was_put() -> Result<()> {
    let cluster = start_cluster(ClusterParams {
        agents: Vec::new(),
        wait_for_slots_ready: false,
        ..ClusterParams::default()
    })
    .await?;

    assert_eq!(
        get_balancer_desired_state(&cluster.client_management).await?,
        BalancerDesiredState::default()
    );

    let desired_state = BalancerDesiredState {
        chat_template_override: None,
        inference_parameters: InferenceParameters::deterministic(),
        model: AgentDesiredModel::None,
        multimodal_projection: AgentDesiredModel::None,
        use_chat_template_override: false,
    };

    cluster
        .client_management
        .put_balancer_desired_state(&desired_state)
        .await
        .map_err(anyhow::Error::new)
        .context("failed to PUT /api/v1/balancer_desired_state")?;

    assert_eq!(
        get_balancer_desired_state(&cluster.client_management).await?,
        desired_state
    );

    cluster.shutdown().await?;

    Ok(())
}
