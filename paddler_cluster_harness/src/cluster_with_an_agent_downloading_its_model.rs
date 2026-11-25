use anyhow::Context as _;
use anyhow::Result;

use paddler_local_http_fixture::local_http_fixture::LocalHttpFixture;
use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;

use crate::cluster::Cluster;
use crate::cluster_desired_state::ClusterDesiredState;
use crate::cluster_params::ClusterParams;
use crate::trickling_model_server::trickling_model_server;

pub struct ClusterWithAnAgentDownloadingItsModel {
    pub cluster: Cluster,
    pub model_server: LocalHttpFixture,
}

impl ClusterWithAnAgentDownloadingItsModel {
    /// # Errors
    ///
    /// Returns an error when the model server cannot start, when the cluster cannot start, and when its first agent does not start downloading the model.
    pub async fn start(cluster_params: ClusterParams) -> Result<Self> {
        let model_server = trickling_model_server()
            .await
            .context("the model server must start")?;
        let mut cluster = Cluster::start_in_process(ClusterParams {
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
