use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use log::error;
use tokio::select;
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;
use trzcina::Service;

use paddler_messaging::agent_desired_state::AgentDesiredState;

use crate::agent_applicable_state_holder::AgentApplicableStateHolder;
use crate::desired_state_reconciler::DesiredStateReconciler;
use crate::desired_state_reconciliation::DesiredStateReconciliation;
use crate::desired_state_reconciliation_attempt::DesiredStateReconciliationAttempt;
use crate::retry_delay::RetryDelay;

pub struct ReconciliationService {
    pub agent_applicable_state_holder: Arc<AgentApplicableStateHolder>,
    pub agent_desired_state_rx: watch::Receiver<AgentDesiredState>,
    pub desired_state_reconciler: DesiredStateReconciler,
}

impl ReconciliationService {
    async fn reconcile_desired_states_until_shutdown(
        self: Box<Self>,
        shutdown: CancellationToken,
    ) -> Result<()> {
        let Self {
            agent_applicable_state_holder,
            mut agent_desired_state_rx,
            desired_state_reconciler,
        } = *self;
        let mut reconciliation_attempt = DesiredStateReconciliationAttempt::Idle;

        let service_outcome = loop {
            select! {
                biased;
                () = shutdown.cancelled() => break Ok(()),
                desired_state_change = agent_desired_state_rx.changed() => {
                    if desired_state_change.is_err() {
                        error!("Agent desired state channel closed, stopping reconciliation service.");

                        break Ok(());
                    }

                    reconciliation_attempt.abandon().await;
                    desired_state_reconciler
                        .slot_aggregated_status
                        .begin_desired_state();
                    reconciliation_attempt = DesiredStateReconciliationAttempt::start(
                        desired_state_reconciler.clone(),
                        agent_desired_state_rx.borrow_and_update().clone(),
                        &shutdown,
                        RetryDelay::Immediately,
                    );
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
                            agent_applicable_state_holder
                                .set_agent_applicable_state(*agent_applicable_state);
                        }
                    }
                }
            }
        };

        reconciliation_attempt.abandon().await;

        service_outcome
    }
}

#[async_trait]
impl Service for ReconciliationService {
    fn name(&self) -> &'static str {
        "agent::reconciliation_service"
    }

    async fn run(self: Box<Self>, shutdown: CancellationToken) -> Result<()> {
        self.reconcile_desired_states_until_shutdown(shutdown).await
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
    use trzcina::Service as _;

    use paddler_agent_status::slot_aggregated_status::SlotAggregatedStatus;
    use paddler_cache_dir::cache_dir::CacheDir;
    use paddler_download_manager::download_manager::DownloadManager;
    use paddler_local_http_fixture::fixture_response::FixtureResponse;
    use paddler_local_http_fixture::local_http_fixture::LocalHttpFixture;
    use paddler_messaging::agent_desired_model::AgentDesiredModel;
    use paddler_messaging::agent_desired_state::AgentDesiredState;
    use paddler_messaging::balancer_desired_state::BalancerDesiredState;
    use paddler_model_source::desired_model_resolver::DesiredModelResolver;
    use paddler_model_source::huggingface_cache_location::HuggingFaceCacheLocation;
    use paddler_model_source::huggingface_default_endpoint::HUGGINGFACE_DEFAULT_ENDPOINT;
    use paddler_model_source::huggingface_hub::HuggingFaceHub;

    use super::ReconciliationService;
    use crate::agent_applicable_state_holder::AgentApplicableStateHolder;
    use crate::desired_state_reconciler::DesiredStateReconciler;

    fn desired_state_with_model(model: AgentDesiredModel) -> AgentDesiredState {
        AgentDesiredState {
            model,
            ..AgentDesiredState::from(BalancerDesiredState::default())
        }
    }

    #[tokio::test(start_paused = true)]
    async fn a_desired_state_arriving_during_retries_is_reconciled_without_waiting() {
        let cache_dir = TempDir::new().unwrap();
        let failing_model_server =
            LocalHttpFixture::start(FixtureResponse::Status(StatusCode::INTERNAL_SERVER_ERROR))
                .await
                .unwrap();
        let agent_applicable_state_holder = Arc::new(AgentApplicableStateHolder::default());
        let mut agent_applicable_state_rx = agent_applicable_state_holder.subscribe();
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
                        download_manager: DownloadManager::new().unwrap(),
                        huggingface_hub: HuggingFaceHub {
                            cache_location: HuggingFaceCacheLocation::HubDefault,
                            endpoint: HUGGINGFACE_DEFAULT_ENDPOINT.to_owned(),
                        },
                    },
                    slot_aggregated_status: Arc::new(SlotAggregatedStatus::new(1)),
                },
            })
            .run(shutdown.clone()),
        );

        agent_desired_state_tx.send_replace(desired_state_with_model(AgentDesiredModel::Uri(
            failing_model_server.url("/model.gguf"),
        )));
        failing_model_server.wait_for_requests(2).await.unwrap();

        let replaced_at = Instant::now();

        agent_desired_state_tx.send_replace(AgentDesiredState::default());
        agent_applicable_state_rx.changed().await.unwrap();

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
}
