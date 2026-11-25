use std::sync::Arc;

use log::Level;
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;
use trzcina::Service as _;

use paddler_agent::delivered_desired_state::DeliveredDesiredState;
use paddler_agent::desired_state_reconciler::DesiredStateReconciler;
use paddler_agent::reconciled_state_holder::ReconciledStateHolder;
use paddler_agent::reconciliation_service::ReconciliationService;
use paddler_agent_status::slot_aggregated_status::SlotAggregatedStatus;
use paddler_cache_dir::cache_dir::CacheDir;
use paddler_capturing_logger::captured_record::CapturedRecord;
use paddler_capturing_logger::capturing_logger::CapturingLogger;
use paddler_download_manager::download_manager::DownloadManager;
use paddler_model_source::constants::HUGGINGFACE_DEFAULT_ENDPOINT;
use paddler_model_source::desired_model_resolver::DesiredModelResolver;
use paddler_model_source::huggingface_cache_location::HuggingFaceCacheLocation;
use paddler_model_source::huggingface_hub::HuggingFaceHub;

#[tokio::test]
async fn the_reconciliation_service_stops_once_its_desired_state_channel_closes() {
    CapturingLogger::install();

    let (delivered_desired_state_tx, delivered_desired_state_rx) =
        watch::channel(DeliveredDesiredState::default());

    drop(delivered_desired_state_tx);

    Box::new(ReconciliationService {
        delivered_desired_state_rx,
        desired_state_reconciler: DesiredStateReconciler {
            desired_model_resolver: DesiredModelResolver {
                cache_dir: CacheDir {
                    explicit: None,
                    home: None,
                    xdg: None,
                },
                download_manager: DownloadManager::new().expect("the model downloader must start"),
                huggingface_client: HuggingFaceHub {
                    cache_location: HuggingFaceCacheLocation::HubDefault,
                    endpoint: HUGGINGFACE_DEFAULT_ENDPOINT.to_owned(),
                }
                .client()
                .expect("the Hugging Face client must start"),
            },
            slot_aggregated_status: Arc::new(SlotAggregatedStatus::new(1)),
        },
        reconciled_state_holder: Arc::new(ReconciledStateHolder::default()),
    })
    .run(CancellationToken::new())
    .await
    .expect("the reconciliation service must stop cleanly once nothing can send it desired states");

    assert!(
        CapturingLogger::records_for("paddler_agent::reconciliation_service")
            .iter()
            .any(|CapturedRecord { level, .. }| *level == Level::Error)
    );
}
