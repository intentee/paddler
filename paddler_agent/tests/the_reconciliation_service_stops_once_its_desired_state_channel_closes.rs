use std::sync::Arc;

use tokio::sync::watch;
use tokio_util::sync::CancellationToken;
use trzcina::Service as _;

use paddler_agent::agent_applicable_state_holder::AgentApplicableStateHolder;
use paddler_agent::desired_state_reconciler::DesiredStateReconciler;
use paddler_agent::reconciliation_service::ReconciliationService;
use paddler_agent_status::slot_aggregated_status::SlotAggregatedStatus;
use paddler_cache_dir::cache_dir::CacheDir;
use paddler_download_manager::download_manager::DownloadManager;
use paddler_messaging::agent_desired_state::AgentDesiredState;
use paddler_model_source::desired_model_resolver::DesiredModelResolver;
use paddler_model_source::huggingface_cache_location::HuggingFaceCacheLocation;
use paddler_model_source::huggingface_default_endpoint::HUGGINGFACE_DEFAULT_ENDPOINT;
use paddler_model_source::huggingface_hub::HuggingFaceHub;

#[tokio::test]
async fn the_reconciliation_service_stops_once_its_desired_state_channel_closes() {
    let (agent_desired_state_tx, agent_desired_state_rx) =
        watch::channel(AgentDesiredState::default());

    drop(agent_desired_state_tx);

    Box::new(ReconciliationService {
        agent_applicable_state_holder: Arc::new(AgentApplicableStateHolder::default()),
        agent_desired_state_rx,
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
    })
    .run(CancellationToken::new())
    .await
    .expect("the reconciliation service must stop cleanly once nothing can send it desired states");
}
