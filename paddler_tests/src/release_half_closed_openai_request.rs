use anyhow::Context as _;
use anyhow::Result;
use futures_util::FutureExt as _;
use futures_util::TryFutureExt as _;
use futures_util::future::ready;
use serde_json::Value;

use paddler_test_cluster_harness::cluster::Cluster;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::half_closed_client::HalfClosedClient;

use crate::start_cluster::start_cluster;

struct HalfClosedOpenAIRequest {
    client: HalfClosedClient,
    cluster: Cluster,
}

impl HalfClosedOpenAIRequest {
    async fn half_close(mut self) -> Result<Self> {
        self.client
            .half_close()
            .await
            .context("the client must half-close its connection")
            .map(|()| self)
    }

    async fn post(cluster: Cluster, path: &str, body: &Value) -> Result<Self> {
        ready(
            cluster
                .balancer
                .compat_openai_addr()
                .context("the balancer must serve the OpenAI compatibility service"),
        )
        .and_then(|compat_openai_addr| {
            HalfClosedClient::post_json_then_half_close(compat_openai_addr, path, body)
                .map(|posted| posted.context("the client must send its OpenAI-compatible request"))
        })
        .map_ok(|client| Self { client, cluster })
        .await
    }

    async fn wait_until_buffered(mut self) -> Result<Self> {
        self.cluster
            .wait_for_buffered_request_count(1)
            .await
            .context("the request must be buffered while no agent is available")
            .map(|_buffered_requests| self)
    }

    async fn wait_until_released(self) -> Result<Cluster> {
        let Self {
            client,
            mut cluster,
        } = self;

        cluster
            .wait_for_buffered_request_count(0)
            .await
            .context("the balancer must release the buffered request once the client half-closes")
            .map(|_buffered_requests| {
                drop(client);

                cluster
            })
    }
}

/// # Errors
///
/// Returns an error when the cluster without agents cannot start, when the client cannot send or half-close its OpenAI-compatible request, when the balancer neither buffers nor releases that request, and when the cluster does not shut down cleanly.
pub async fn release_half_closed_openai_request(path: &str, body: &Value) -> Result<()> {
    start_cluster(ClusterParams::without_agents())
        .map(|started| started.context("a cluster without agents must start"))
        .and_then(|cluster| HalfClosedOpenAIRequest::post(cluster, path, body))
        .and_then(HalfClosedOpenAIRequest::wait_until_buffered)
        .and_then(HalfClosedOpenAIRequest::half_close)
        .and_then(HalfClosedOpenAIRequest::wait_until_released)
        .and_then(|cluster: Cluster| {
            cluster.shutdown().map(|shut_down: Result<()>| {
                shut_down.context("the cluster must shut down cleanly")
            })
        })
        .await
}
