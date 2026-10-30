use std::future::Future;

use anyhow::Result;
use futures_util::TryFutureExt as _;
use futures_util::future::ready;

use paddler_test_cluster_harness::cluster::Cluster;

use crate::decision_cluster_params::DecisionClusterParams;
use crate::start_cluster::start_cluster;

pub fn start_decision_cluster(
    decision_cluster_params: DecisionClusterParams,
) -> impl Future<Output = Result<Cluster>> {
    ready(decision_cluster_params.into_cluster_params())
        .err_into()
        .and_then(start_cluster)
}
