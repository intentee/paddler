use anyhow::Result;

use paddler_test_cluster_harness::cluster::Cluster;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::model_card::ministral_3_3b_reasoning::ministral_3_3b_reasoning;

use crate::ministral_3_cluster_params::Ministral3ClusterParams;
use crate::start_cluster::start_cluster;

pub async fn start_cluster_with_ministral_3(
    Ministral3ClusterParams { agents }: Ministral3ClusterParams,
) -> Result<Cluster> {
    start_cluster(ClusterParams {
        agents,
        desired_state: Some(ministral_3_3b_reasoning().into_desired_state()),
        wait_for_slots_ready: true,
        ..ClusterParams::default()
    })
    .await
}
