use std::sync::Arc;

use tempfile::TempDir;
use tokio::spawn;
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;
use trzcina::Service as _;

use paddler_agent::agent_applicable_state_holder::AgentApplicableStateHolder;
use paddler_agent::desired_state_reconciler::DesiredStateReconciler;
use paddler_agent::reconciliation_service::ReconciliationService;
use paddler_agent_status::slot_aggregated_status::SlotAggregatedStatus;
use paddler_blocking_thread_fixture::occupied_blocking_thread::OccupiedBlockingThread;
use paddler_blocking_thread_fixture::single_blocking_thread_runtime::single_blocking_thread_runtime;
use paddler_cache_dir::cache_dir::CacheDir;
use paddler_download_manager::download_manager::DownloadManager;
use paddler_local_http_fixture::fixture_response::FixtureResponse;
use paddler_local_http_fixture::local_http_fixture::LocalHttpFixture;
use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::agent_desired_state::AgentDesiredState;
use paddler_messaging::subscribes_to_updates::SubscribesToUpdates as _;
use paddler_model_source::desired_model_resolver::DesiredModelResolver;
use paddler_model_source::huggingface_cache_location::HuggingFaceCacheLocation;
use paddler_model_source::huggingface_default_endpoint::HUGGINGFACE_DEFAULT_ENDPOINT;
use paddler_model_source::huggingface_hub::HuggingFaceHub;

use crate::reconciliation_after_replacement::ReconciliationAfterReplacement;

pub struct ReconciliationReplacedBeforeItsWorkerRuns {
    pub replaced_desired_state: AgentDesiredState,
}

impl ReconciliationReplacedBeforeItsWorkerRuns {
    pub fn run(self) -> ReconciliationAfterReplacement {
        single_blocking_thread_runtime()
            .expect("the test runtime must build")
            .block_on(self.run_on_one_blocking_thread())
    }

    async fn run_on_one_blocking_thread(self) -> ReconciliationAfterReplacement {
        let cache_dir = TempDir::new().expect("the download cache must be creatable");
        let stalled_model_server = LocalHttpFixture::start(FixtureResponse::StallBeforeHeaders)
            .await
            .expect("the stalled model server must start");
        let occupied_blocking_thread = OccupiedBlockingThread::occupy();
        let agent_applicable_state_holder = Arc::new(AgentApplicableStateHolder::default());
        let slot_aggregated_status = Arc::new(SlotAggregatedStatus::new(1));
        let mut status_updates = slot_aggregated_status.subscribe_to_updates();
        let (agent_desired_state_tx, agent_desired_state_rx) =
            watch::channel(AgentDesiredState::default());
        let shutdown = CancellationToken::new();
        let reconciliation = spawn(
            Box::new(ReconciliationService {
                agent_applicable_state_holder: agent_applicable_state_holder.clone(),
                agent_desired_state_rx,
                desired_state_reconciler: DesiredStateReconciler {
                    desired_model_resolver: DesiredModelResolver {
                        cache_dir: CacheDir {
                            explicit: Some(cache_dir.path().to_path_buf()),
                            home: None,
                            xdg: None,
                        },
                        download_manager: DownloadManager::new()
                            .expect("the model downloader must start"),
                        huggingface_client: HuggingFaceHub {
                            cache_location: HuggingFaceCacheLocation::HubDefault,
                            endpoint: HUGGINGFACE_DEFAULT_ENDPOINT.to_owned(),
                        }
                        .client()
                        .expect("the Hugging Face client must start"),
                    },
                    slot_aggregated_status: slot_aggregated_status.clone(),
                },
            })
            .run(shutdown.clone()),
        );

        status_updates.borrow_and_update();
        agent_desired_state_tx.send_replace(self.replaced_desired_state);
        status_updates
            .changed()
            .await
            .expect("the agent must begin the replaced desired state");
        agent_desired_state_tx.send_replace(AgentDesiredState {
            model: AgentDesiredModel::Uri(stalled_model_server.url("/model.gguf")),
            ..AgentDesiredState::default()
        });
        occupied_blocking_thread
            .release()
            .await
            .expect("the blocking thread occupant must be released");
        stalled_model_server
            .wait_for_requests(1)
            .await
            .expect("the replacement must start downloading after the replaced worker ran");
        shutdown.cancel();
        reconciliation
            .await
            .expect("the reconciliation service must not panic")
            .expect("the reconciliation service must stop cleanly");
        stalled_model_server
            .stop()
            .await
            .expect("the stalled model server must not panic")
            .expect("the stalled model server must serve every request");

        ReconciliationAfterReplacement {
            agent_applicable_state_holder,
            slot_aggregated_status,
        }
    }
}
