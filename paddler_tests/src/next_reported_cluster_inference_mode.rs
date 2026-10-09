use anyhow::Result;
use tokio::sync::watch::Receiver;

use paddler_client::inference_socket::reported_cluster_inference_mode::ReportedClusterInferenceMode;

pub async fn next_reported_cluster_inference_mode(
    cluster_inference_mode_rx: &mut Receiver<ReportedClusterInferenceMode>,
) -> Result<ReportedClusterInferenceMode> {
    cluster_inference_mode_rx.changed().await?;

    Ok(*cluster_inference_mode_rx.borrow_and_update())
}
