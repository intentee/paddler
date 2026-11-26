use std::convert::Infallible;
use std::mem::replace;
use std::sync::Arc;

use log::error;
use tokio::select;
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;
use trzcina::service::Service;

use crate::delivered_desired_state::DeliveredDesiredState;
use crate::desired_state_reconciler::DesiredStateReconciler;
use crate::desired_state_reconciliation::DesiredStateReconciliation;
use crate::desired_state_reconciliation_attempt::DesiredStateReconciliationAttempt;
use crate::reconciled_state::ReconciledState;
use crate::reconciled_state_holder::ReconciledStateHolder;
use crate::reconciliation_phase::ReconciliationPhase;
use crate::retry_delay::RetryDelay;

pub struct ReconciliationService {
    pub delivered_desired_state_rx: watch::Receiver<DeliveredDesiredState>,
    pub desired_state_reconciler: DesiredStateReconciler,
    pub reconciled_state_holder: Arc<ReconciledStateHolder>,
}

impl ReconciliationService {
    async fn reconcile_desired_states_until_shutdown(self: Box<Self>, shutdown: CancellationToken) {
        let Self {
            mut delivered_desired_state_rx,
            desired_state_reconciler,
            reconciled_state_holder,
        } = *self;
        let mut delivered_desired_state = delivered_desired_state_rx.borrow().clone();
        let mut reconciliation_attempt = DesiredStateReconciliationAttempt::Idle;
        let mut reconciliation_phase = ReconciliationPhase::NotStarted;

        loop {
            select! {
                biased;
                () = shutdown.cancelled() => break,
                delivery = delivered_desired_state_rx.changed() => {
                    if delivery.is_err() {
                        error!("Agent desired state channel closed, stopping reconciliation service.");

                        break;
                    }

                    let previously_delivered_desired_state = replace(
                        &mut delivered_desired_state,
                        delivered_desired_state_rx.borrow_and_update().clone(),
                    );
                    let delivers_the_same_desired_state = previously_delivered_desired_state
                        .desired_state
                        == delivered_desired_state.desired_state;

                    match &reconciliation_phase {
                        ReconciliationPhase::Reconciled(reconciled_applicable_state)
                            if delivers_the_same_desired_state =>
                        {
                            reconciled_state_holder.set_reconciled_state(ReconciledState {
                                agent_applicable_state: reconciled_applicable_state.clone(),
                                desired_state_version: delivered_desired_state.version,
                            });
                        }
                        ReconciliationPhase::Reconciling if delivers_the_same_desired_state => {}
                        ReconciliationPhase::NotStarted
                        | ReconciliationPhase::Reconciled(_)
                        | ReconciliationPhase::Reconciling => {
                            reconciliation_attempt.abandon().await;
                            desired_state_reconciler
                                .slot_aggregated_status
                                .begin_desired_state();
                            reconciliation_attempt = DesiredStateReconciliationAttempt::start(
                                desired_state_reconciler.clone(),
                                delivered_desired_state.desired_state.clone(),
                                &shutdown,
                                RetryDelay::Immediately,
                            );
                            reconciliation_phase = ReconciliationPhase::Reconciling;
                        }
                    }
                }
                desired_state_reconciliation = reconciliation_attempt.finish() => {
                    match desired_state_reconciliation {
                        DesiredStateReconciliation::Pending(agent_desired_state) => {
                            reconciliation_attempt = DesiredStateReconciliationAttempt::start(
                                desired_state_reconciler.clone(),
                                *agent_desired_state,
                                &shutdown,
                                RetryDelay::AfterRetryInterval,
                            );
                        }
                        DesiredStateReconciliation::Reconciled(agent_applicable_state) => {
                            reconciled_state_holder.set_reconciled_state(ReconciledState {
                                agent_applicable_state: (*agent_applicable_state).clone(),
                                desired_state_version: delivered_desired_state.version,
                            });
                            reconciliation_phase =
                                ReconciliationPhase::Reconciled(*agent_applicable_state);
                        }
                    }
                }
            }
        }

        reconciliation_attempt.abandon().await;
    }
}

impl Service for ReconciliationService {
    type Error = Infallible;

    fn name(&self) -> &'static str {
        "agent::reconciliation_service"
    }

    async fn run(self: Box<Self>, shutdown: CancellationToken) -> Result<(), Infallible> {
        self.reconcile_desired_states_until_shutdown(shutdown).await;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use http::StatusCode;
    use tempfile::TempDir;
    use tokio::spawn;
    use tokio::sync::watch;
    use tokio::time::Instant;
    use tokio_util::sync::CancellationToken;
    use trzcina::service::Service as _;

    use paddler_agent_status::slot_aggregated_status::SlotAggregatedStatus;
    use paddler_cache_dir::cache_dir::CacheDir;
    use paddler_download_manager::download_manager::DownloadManager;
    use paddler_local_http_fixture::fixture_response::FixtureResponse;
    use paddler_local_http_fixture::local_http_fixture::LocalHttpFixture;
    use paddler_messaging::agent_desired_model::AgentDesiredModel;
    use paddler_messaging::agent_desired_state::AgentDesiredState;
    use paddler_messaging::desired_state_version::DesiredStateVersion;
    use paddler_messaging::subscribes_to_updates::SubscribesToUpdates as _;
    use paddler_model_source::desired_model_resolver::DesiredModelResolver;
    use paddler_model_source::huggingface_cache_location::HuggingFaceCacheLocation;
    use paddler_model_source::huggingface_endpoint::HuggingFaceEndpoint;
    use paddler_model_source::huggingface_hub::HuggingFaceHub;

    use super::ReconciliationService;
    use crate::delivered_desired_state::DeliveredDesiredState;
    use crate::desired_state_reconciler::DesiredStateReconciler;
    use crate::reconciled_state::ReconciledState;
    use crate::reconciled_state_holder::ReconciledStateHolder;
    use crate::reconciliation_progress::ReconciliationProgress;

    fn reconciliation_service_caching_in(
        cache_dir: &TempDir,
        delivered_desired_state_rx: watch::Receiver<DeliveredDesiredState>,
        reconciled_state_holder: Arc<ReconciledStateHolder>,
        slot_aggregated_status: Arc<SlotAggregatedStatus>,
    ) -> ReconciliationService {
        ReconciliationService {
            delivered_desired_state_rx,
            desired_state_reconciler: DesiredStateReconciler {
                desired_model_resolver: DesiredModelResolver {
                    cache_dir: CacheDir {
                        explicit: Some(cache_dir.path().to_path_buf()),
                        home: None,
                        xdg: None,
                    },
                    download_manager: DownloadManager::new().unwrap(),
                    huggingface_client: HuggingFaceHub {
                        cache_location: HuggingFaceCacheLocation::HubDefault,
                        endpoint: HuggingFaceEndpoint::HubDefault,
                    }
                    .client()
                    .unwrap(),
                },
                slot_aggregated_status,
            },
            reconciled_state_holder,
        }
    }

    fn desired_state_downloading_from(model_server: &LocalHttpFixture) -> AgentDesiredState {
        AgentDesiredState {
            model: AgentDesiredModel::Uri(model_server.url("/model.gguf")),
            ..AgentDesiredState::default()
        }
    }

    #[tokio::test(start_paused = true)]
    async fn a_desired_state_arriving_during_retries_is_reconciled_without_waiting() {
        let cache_dir = TempDir::new().unwrap();
        let failing_model_server =
            LocalHttpFixture::start(FixtureResponse::Status(StatusCode::INTERNAL_SERVER_ERROR))
                .await
                .unwrap();
        let reconciled_state_holder = Arc::new(ReconciledStateHolder::default());
        let mut reconciled_state_rx = reconciled_state_holder.subscribe();
        let (delivered_desired_state_tx, delivered_desired_state_rx) =
            watch::channel(DeliveredDesiredState::default());
        let shutdown = CancellationToken::new();
        let reconciliation = spawn(
            Box::new(reconciliation_service_caching_in(
                &cache_dir,
                delivered_desired_state_rx,
                reconciled_state_holder.clone(),
                Arc::new(SlotAggregatedStatus::new(1)),
            ))
            .run(shutdown.clone()),
        );

        delivered_desired_state_tx.send_replace(DeliveredDesiredState {
            desired_state: desired_state_downloading_from(&failing_model_server),
            version: DesiredStateVersion::Version(1),
        });
        failing_model_server.wait_for_requests(2).await.unwrap();

        let replaced_at = Instant::now();

        delivered_desired_state_tx.send_replace(DeliveredDesiredState {
            desired_state: AgentDesiredState::default(),
            version: DesiredStateVersion::Version(2),
        });
        reconciled_state_rx.changed().await.unwrap();

        assert_eq!(Instant::now(), replaced_at);
        assert_eq!(failing_model_server.request_count(), 2);

        shutdown.cancel();
        reconciliation.await.unwrap().unwrap();
        failing_model_server
            .stop()
            .await
            .expect("the fixture task must not panic")
            .expect("the fixture must serve every request");
    }

    #[tokio::test(start_paused = true)]
    async fn a_desired_state_delivered_again_while_it_retries_keeps_its_retry_schedule() {
        let cache_dir = TempDir::new().unwrap();
        let failing_model_server =
            LocalHttpFixture::start(FixtureResponse::Status(StatusCode::INTERNAL_SERVER_ERROR))
                .await
                .unwrap();
        let (delivered_desired_state_tx, delivered_desired_state_rx) =
            watch::channel(DeliveredDesiredState::default());
        let shutdown = CancellationToken::new();
        let reconciliation = spawn(
            Box::new(reconciliation_service_caching_in(
                &cache_dir,
                delivered_desired_state_rx,
                Arc::new(ReconciledStateHolder::default()),
                Arc::new(SlotAggregatedStatus::new(1)),
            ))
            .run(shutdown.clone()),
        );

        delivered_desired_state_tx.send_replace(DeliveredDesiredState {
            desired_state: desired_state_downloading_from(&failing_model_server),
            version: DesiredStateVersion::Version(1),
        });
        failing_model_server.wait_for_requests(1).await.unwrap();

        let delivered_again_at = Instant::now();

        delivered_desired_state_tx.send_replace(DeliveredDesiredState {
            desired_state: desired_state_downloading_from(&failing_model_server),
            version: DesiredStateVersion::Version(2),
        });
        failing_model_server.wait_for_requests(2).await.unwrap();

        assert!(Instant::now() > delivered_again_at);

        shutdown.cancel();
        reconciliation.await.unwrap().unwrap();
        failing_model_server
            .stop()
            .await
            .expect("the fixture task must not panic")
            .expect("the fixture must serve every request");
    }

    #[tokio::test(start_paused = true)]
    async fn a_reconciled_desired_state_delivered_again_is_acknowledged_without_beginning_it_again()
    {
        let cache_dir = TempDir::new().unwrap();
        let reconciled_state_holder = Arc::new(ReconciledStateHolder::default());
        let mut reconciled_state_rx = reconciled_state_holder.subscribe();
        let slot_aggregated_status = Arc::new(SlotAggregatedStatus::new(1));
        let (delivered_desired_state_tx, delivered_desired_state_rx) =
            watch::channel(DeliveredDesiredState::default());
        let shutdown = CancellationToken::new();
        let reconciliation = spawn(
            Box::new(reconciliation_service_caching_in(
                &cache_dir,
                delivered_desired_state_rx,
                reconciled_state_holder.clone(),
                slot_aggregated_status.clone(),
            ))
            .run(shutdown.clone()),
        );

        reconciled_state_rx.borrow_and_update();
        delivered_desired_state_tx.send_replace(DeliveredDesiredState {
            desired_state: AgentDesiredState::default(),
            version: DesiredStateVersion::Version(1),
        });
        reconciled_state_rx.changed().await.unwrap();

        let mut status_updates = slot_aggregated_status.subscribe_to_updates();

        status_updates.borrow_and_update();
        delivered_desired_state_tx.send_replace(DeliveredDesiredState {
            desired_state: AgentDesiredState::default(),
            version: DesiredStateVersion::Version(2),
        });
        reconciled_state_rx.changed().await.unwrap();

        assert!(matches!(
            *reconciled_state_rx.borrow(),
            ReconciliationProgress::Reconciled(ReconciledState {
                desired_state_version: DesiredStateVersion::Version(2),
                ..
            })
        ));
        assert!(!status_updates.has_changed().unwrap());

        shutdown.cancel();
        reconciliation.await.unwrap().unwrap();
    }
}
