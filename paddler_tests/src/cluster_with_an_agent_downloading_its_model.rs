use anyhow::Context as _;
use anyhow::Result;

use paddler_local_http_fixture::fixture_response::FixtureResponse;
use paddler_local_http_fixture::local_http_fixture::LocalHttpFixture;
use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_test_cluster_harness::cluster::Cluster;
use paddler_test_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_test_cluster_harness::cluster_params::ClusterParams;

use crate::constants::BYTE_INTERVAL_WITHIN_THE_READ_TIMEOUT;
use crate::start_cluster::start_cluster;

pub struct ClusterWithAnAgentDownloadingItsModel {
    pub cluster: Cluster,
    pub model_server: LocalHttpFixture,
}

impl ClusterWithAnAgentDownloadingItsModel {
    /// # Errors
    ///
    /// Returns an error when the model server cannot start, when the cluster cannot start, and when its first agent does not start downloading the model.
    pub async fn start(cluster_params: ClusterParams) -> Result<Self> {
        let model_server = LocalHttpFixture::start(FixtureResponse::EndlessTrickle {
            byte_interval: BYTE_INTERVAL_WITHIN_THE_READ_TIMEOUT,
        })
        .await
        .context("the model server must start")?;
        let mut cluster = start_cluster(ClusterParams {
            desired_state: ClusterDesiredState::Apply(Box::new(BalancerDesiredState {
                model: AgentDesiredModel::Uri(model_server.url("/model.gguf")),
                ..BalancerDesiredState::default()
            })),
            wait_for_slots_ready: false,
            ..cluster_params
        })
        .await?;

        cluster
            .wait_for_first_agent_to_download()
            .await
            .context("the first agent must start downloading its model")?;

        Ok(Self {
            cluster,
            model_server,
        })
    }

    /// # Errors
    ///
    /// Returns an error when the cluster does not shut down cleanly, and when the model server fails or does not stop.
    pub async fn shutdown(self) -> Result<()> {
        let Self {
            cluster,
            model_server,
        } = self;

        cluster.shutdown().await?;
        model_server
            .stop()
            .await
            .context("the model server task must not panic")?
            .context("the model server must serve every request")
    }
}
