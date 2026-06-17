use anyhow::Result;
use paddler_cluster::cluster::Cluster;
use paddler_model_card::ministral_3_cluster_params::Ministral3ClusterParams;

use crate::in_process_cluster_backend::InProcessClusterBackend;

pub async fn start_cluster_with_ministral_3(params: Ministral3ClusterParams) -> Result<Cluster> {
    Cluster::start(
        &InProcessClusterBackend::default(),
        params.into_cluster_params(),
    )
    .await
}
