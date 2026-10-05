use std::future::Future;

use anyhow::Context as _;
use anyhow::Result;
use tokio::spawn;
use tokio_util::sync::CancellationToken;

use paddler_test_cluster_harness::cluster::Cluster;

use crate::qwen3_desired_state_with_embeddings::qwen3_desired_state_with_embeddings;

pub async fn load_qwen3_after_request_is_buffered<TRequest, TOutput>(
    cluster: &mut Cluster,
    request: TRequest,
    enable_embeddings: bool,
) -> Result<TOutput>
where
    TRequest: Future<Output = Result<TOutput>> + Send + 'static,
    TOutput: Send + 'static,
{
    let buffered_request = spawn(request);

    cluster
        .wait_for_buffered_request_count(1)
        .await
        .context("the request must wait in the buffer while the agent has no model")?;

    cluster
        .client_management
        .put_balancer_desired_state(
            CancellationToken::new(),
            &qwen3_desired_state_with_embeddings(enable_embeddings),
        )
        .await?;

    buffered_request
        .await
        .context("the buffered request task must not panic")?
}
