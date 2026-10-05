use std::time::Duration;

use anyhow::Context as _;
use anyhow::Result;
use paddler_client::client_management::ClientManagement;
use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::agent_desired_state::AgentDesiredState;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::start_cluster::start_cluster;
use tokio::time::sleep;
use tokio::time::timeout;
use tokio_util::sync::CancellationToken;

const RECONCILIATION_PROBE_INTERVAL: Duration = Duration::from_millis(20);
const RECONCILIATION_TIMEOUT: Duration = Duration::from_secs(5);

async fn wait_for_applicable_state(
    client_management: &ClientManagement,
) -> Result<AgentDesiredState> {
    loop {
        let applicable_state = client_management
            .get_balancer_applicable_state(CancellationToken::new())
            .await
            .map_err(anyhow::Error::new)
            .context("failed to GET /api/v1/balancer_applicable_state")?;

        if let Some(agent_desired_state) = applicable_state {
            return Ok(agent_desired_state);
        }

        sleep(RECONCILIATION_PROBE_INTERVAL).await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn management_applicable_state_reflects_desired_state_after_reconciliation() -> Result<()> {
    let cluster = start_cluster(ClusterParams {
        agents: Vec::new(),
        wait_for_slots_ready: false,
        ..ClusterParams::default()
    })
    .await?;

    let agent_desired_state = timeout(
        RECONCILIATION_TIMEOUT,
        wait_for_applicable_state(&cluster.client_management),
    )
    .await
    .context("balancer did not reconcile its desired state in time")??;

    assert_eq!(agent_desired_state.model, AgentDesiredModel::None);
    assert_eq!(agent_desired_state.chat_template_override, None);

    cluster.shutdown().await?;

    Ok(())
}
