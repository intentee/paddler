use std::io::Result as IoResult;
use std::sync::Arc;

use anyhow::Result as AnyhowResult;
use futures_util::TryFutureExt as _;
use futures_util::future::ready;
use tempfile::TempDir;
use tokio::spawn;
use tokio::sync::watch;
use tokio::task::JoinHandle;
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
use paddler_model_source::constants::HUGGINGFACE_DEFAULT_ENDPOINT;
use paddler_model_source::desired_model_resolver::DesiredModelResolver;
use paddler_model_source::huggingface_cache_location::HuggingFaceCacheLocation;
use paddler_model_source::huggingface_hub::HuggingFaceHub;

use crate::agent_tests_error::AgentTestsError;
use crate::reconciliation_after_replacement::ReconciliationAfterReplacement;

fn desired_model_resolver_caching_in(
    cache_dir: &TempDir,
) -> Result<DesiredModelResolver, AgentTestsError> {
    DownloadManager::new()
        .map_err(AgentTestsError::ModelDownloaderUnstartable)
        .and_then(|download_manager| {
            HuggingFaceHub {
                cache_location: HuggingFaceCacheLocation::HubDefault,
                endpoint: HUGGINGFACE_DEFAULT_ENDPOINT.to_owned(),
            }
            .client()
            .map_err(AgentTestsError::HuggingFaceClientUnstartable)
            .map(|huggingface_client| DesiredModelResolver {
                cache_dir: CacheDir::from_explicit_path(cache_dir.path()),
                download_manager,
                huggingface_client,
            })
        })
}

struct ReplacementDownloading {
    agent_applicable_state_holder: Arc<AgentApplicableStateHolder>,
    agent_desired_state_tx: watch::Sender<AgentDesiredState>,
    cache_dir: TempDir,
    reconciliation: JoinHandle<AnyhowResult<()>>,
    shutdown: CancellationToken,
    slot_aggregated_status: Arc<SlotAggregatedStatus>,
    stalled_model_server: LocalHttpFixture,
    status_updates: watch::Receiver<()>,
}

impl ReplacementDownloading {
    async fn until_the_download_starts(self) -> Result<Self, AgentTestsError> {
        let download_started = self.stalled_model_server.wait_for_requests(1).await;

        download_started
            .map(|()| self)
            .map_err(AgentTestsError::ReplacementDownloadNeverStarted)
    }

    async fn stop(self) -> Result<ReconciliationAfterReplacement, AgentTestsError> {
        let Self {
            agent_applicable_state_holder,
            agent_desired_state_tx,
            cache_dir,
            reconciliation,
            shutdown,
            slot_aggregated_status,
            stalled_model_server,
            status_updates,
        } = self;

        shutdown.cancel();

        let stopped = reconciliation
            .map_err(AgentTestsError::ReconciliationServicePanicked)
            .and_then(|service_outcome| {
                ready(service_outcome.map_err(AgentTestsError::ReconciliationServiceFailed))
            })
            .and_then(|()| {
                stalled_model_server
                    .stop()
                    .map_err(AgentTestsError::StalledModelServerPanicked)
            })
            .and_then(|server_outcome: IoResult<()>| {
                ready(server_outcome.map_err(AgentTestsError::StalledModelServerFailed))
            })
            .map_ok(|()| ReconciliationAfterReplacement {
                agent_applicable_state_holder,
                slot_aggregated_status,
            })
            .await;

        drop(status_updates);
        drop(agent_desired_state_tx);
        drop(cache_dir);

        stopped
    }
}

struct ReplacementPending {
    agent_applicable_state_holder: Arc<AgentApplicableStateHolder>,
    agent_desired_state_tx: watch::Sender<AgentDesiredState>,
    cache_dir: TempDir,
    occupied_blocking_thread: OccupiedBlockingThread,
    reconciliation: JoinHandle<AnyhowResult<()>>,
    shutdown: CancellationToken,
    slot_aggregated_status: Arc<SlotAggregatedStatus>,
    stalled_model_server: LocalHttpFixture,
    status_updates: watch::Receiver<()>,
}

impl ReplacementPending {
    async fn begin_the_replaced_state(
        mut self,
        replaced_desired_state: AgentDesiredState,
    ) -> Result<Self, AgentTestsError> {
        self.status_updates.borrow_and_update();
        self.agent_desired_state_tx
            .send_replace(replaced_desired_state);

        let replaced_state_began = self.status_updates.changed().await;

        replaced_state_began
            .map(|()| self)
            .map_err(AgentTestsError::ReplacedDesiredStateNeverBegun)
    }

    fn start(
        cache_dir: TempDir,
        stalled_model_server: LocalHttpFixture,
    ) -> Result<Self, AgentTestsError> {
        let occupied_blocking_thread = OccupiedBlockingThread::occupy();
        let agent_applicable_state_holder = Arc::new(AgentApplicableStateHolder::default());
        let slot_aggregated_status = Arc::new(SlotAggregatedStatus::new(1));
        let status_updates = slot_aggregated_status.subscribe_to_updates();
        let (agent_desired_state_tx, agent_desired_state_rx) =
            watch::channel(AgentDesiredState::default());
        let shutdown = CancellationToken::new();

        desired_model_resolver_caching_in(&cache_dir).map(|desired_model_resolver| {
            let reconciliation = spawn(
                Box::new(ReconciliationService {
                    agent_applicable_state_holder: agent_applicable_state_holder.clone(),
                    agent_desired_state_rx,
                    desired_state_reconciler: DesiredStateReconciler {
                        desired_model_resolver,
                        slot_aggregated_status: slot_aggregated_status.clone(),
                    },
                })
                .run(shutdown.clone()),
            );

            Self {
                agent_applicable_state_holder,
                agent_desired_state_tx,
                cache_dir,
                occupied_blocking_thread,
                reconciliation,
                shutdown,
                slot_aggregated_status,
                stalled_model_server,
                status_updates,
            }
        })
    }

    async fn replace_it_with_the_stalled_download(
        self,
    ) -> Result<ReplacementDownloading, AgentTestsError> {
        let Self {
            agent_applicable_state_holder,
            agent_desired_state_tx,
            cache_dir,
            occupied_blocking_thread,
            reconciliation,
            shutdown,
            slot_aggregated_status,
            stalled_model_server,
            status_updates,
        } = self;

        agent_desired_state_tx.send_replace(AgentDesiredState {
            model: AgentDesiredModel::Uri(stalled_model_server.url("/model.gguf")),
            ..AgentDesiredState::default()
        });

        occupied_blocking_thread
            .release()
            .map_err(AgentTestsError::BlockingThreadOccupantUnreleased)
            .map_ok(|()| ReplacementDownloading {
                agent_applicable_state_holder,
                agent_desired_state_tx,
                cache_dir,
                reconciliation,
                shutdown,
                slot_aggregated_status,
                stalled_model_server,
                status_updates,
            })
            .await
    }
}

pub struct ReconciliationReplacedBeforeItsWorkerRuns {
    pub replaced_desired_state: AgentDesiredState,
}

impl ReconciliationReplacedBeforeItsWorkerRuns {
    /// # Errors
    ///
    /// Returns [`AgentTestsError::TestRuntimeUnbuildable`] when the single blocking thread runtime cannot be built, [`AgentTestsError::DownloadCacheUncreatable`] when the download cache cannot be created, [`AgentTestsError::StalledModelServerUnstartable`] when the stalled model server cannot start, [`AgentTestsError::ModelDownloaderUnstartable`] or [`AgentTestsError::HuggingFaceClientUnstartable`] when the model resolver cannot start, [`AgentTestsError::ReplacedDesiredStateNeverBegun`] when the agent never begins the replaced desired state, [`AgentTestsError::BlockingThreadOccupantUnreleased`] when the blocking thread occupant cannot be released, [`AgentTestsError::ReplacementDownloadNeverStarted`] when the replacement never starts downloading, [`AgentTestsError::ReconciliationServicePanicked`] or [`AgentTestsError::ReconciliationServiceFailed`] when the reconciliation service does not stop cleanly, and [`AgentTestsError::StalledModelServerPanicked`] or [`AgentTestsError::StalledModelServerFailed`] when the stalled model server does not stop cleanly.
    pub fn run(self) -> Result<ReconciliationAfterReplacement, AgentTestsError> {
        single_blocking_thread_runtime()
            .map_err(AgentTestsError::TestRuntimeUnbuildable)
            .and_then(|runtime| runtime.block_on(self.run_on_one_blocking_thread()))
    }

    async fn run_on_one_blocking_thread(
        self,
    ) -> Result<ReconciliationAfterReplacement, AgentTestsError> {
        let Self {
            replaced_desired_state,
        } = self;

        ready(TempDir::new().map_err(AgentTestsError::DownloadCacheUncreatable))
            .and_then(|cache_dir| {
                LocalHttpFixture::start(FixtureResponse::StallBeforeHeaders)
                    .map_err(AgentTestsError::StalledModelServerUnstartable)
                    .and_then(|stalled_model_server| {
                        ready(ReplacementPending::start(cache_dir, stalled_model_server))
                    })
            })
            .and_then(|replacement_pending| {
                replacement_pending.begin_the_replaced_state(replaced_desired_state)
            })
            .and_then(ReplacementPending::replace_it_with_the_stalled_download)
            .and_then(ReplacementDownloading::until_the_download_starts)
            .and_then(ReplacementDownloading::stop)
            .await
    }
}
