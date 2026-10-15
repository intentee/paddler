use anyhow::Result;
use async_trait::async_trait;
use log::error;
use tokio::select;
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;
use trzcina::Service;

use paddler_messaging::agent_desired_state::AgentDesiredState;

use crate::desired_state_reconciler::DesiredStateReconciler;
use crate::desired_state_reconciliation::DesiredStateReconciliation;
use crate::desired_state_reconciliation_attempt::DesiredStateReconciliationAttempt;
use crate::retry_delay::RetryDelay;

pub struct ReconciliationService {
    pub agent_desired_state_rx: watch::Receiver<AgentDesiredState>,
    pub desired_state_reconciler: DesiredStateReconciler,
}

#[async_trait]
impl Service for ReconciliationService {
    fn name(&self) -> &'static str {
        "agent::reconciliation_service"
    }

    async fn run(self: Box<Self>, shutdown: CancellationToken) -> Result<()> {
        let Self {
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

                    reconciliation_attempt.abandon();
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
                    if let DesiredStateReconciliation::Pending(agent_desired_state) = desired_state_reconciliation {
                        reconciliation_attempt = DesiredStateReconciliationAttempt::start(
                            desired_state_reconciler.clone(),
                            *agent_desired_state,
                            &shutdown,
                            RetryDelay::AfterRetryInterval,
                        );
                    }
                }
            }
        };

        reconciliation_attempt.abandon();

        service_outcome
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
    use paddler_local_http_fixture::fixture_response::FixtureResponse;
    use paddler_local_http_fixture::local_http_fixture::LocalHttpFixture;
    use paddler_messaging::agent_desired_model::AgentDesiredModel;
    use paddler_messaging::agent_desired_state::AgentDesiredState;
    use paddler_messaging::balancer_desired_state::BalancerDesiredState;
    use paddler_model_source::huggingface_model_reference::HuggingFaceModelReference;
    use paddler_model_source::model_source::ModelSource;

    use super::ReconciliationService;
    use crate::agent_applicable_state::AgentApplicableState;
    use crate::agent_applicable_state_holder::AgentApplicableStateHolder;
    use crate::desired_state_reconciler::DesiredStateReconciler;
    use crate::desired_state_reconciliation::DesiredStateReconciliation;

    fn desired_state_with_model(model: AgentDesiredModel) -> AgentDesiredState {
        AgentDesiredState {
            model,
            ..AgentDesiredState::from(BalancerDesiredState::default())
        }
    }

    fn desired_state_reconciler() -> DesiredStateReconciler {
        DesiredStateReconciler {
            agent_applicable_state_holder: Arc::new(AgentApplicableStateHolder::default()),
            cache_dir: CacheDir::from_process_env(),
            slot_aggregated_status: Arc::new(SlotAggregatedStatus::new(1)),
        }
    }

    #[tokio::test]
    async fn a_missing_local_model_leaves_the_state_unconverted() {
        let reconciler = desired_state_reconciler();
        let desired_state = desired_state_with_model(
            ModelSource::LocalToAgent(
                "/paddler-nonexistent-model-for-reconciliation.gguf".to_owned(),
            )
            .into_agent_desired_model(),
        );

        let reconciliation = reconciler
            .reconcile(&CancellationToken::new(), desired_state.clone())
            .await;

        assert_eq!(
            reconciliation,
            DesiredStateReconciliation::Pending(Box::new(desired_state))
        );
        assert_eq!(
            reconciler
                .agent_applicable_state_holder
                .get_agent_applicable_state(),
            AgentApplicableState::default()
        );
    }

    #[tokio::test]
    async fn a_model_download_cancelled_mid_conversion_leaves_the_state_unconverted() {
        let cancellation_token = CancellationToken::new();

        cancellation_token.cancel();

        let reconciler = desired_state_reconciler();
        let desired_state = desired_state_with_model(
            ModelSource::HuggingFace(HuggingFaceModelReference {
                filename: "model.gguf".to_owned(),
                repo_id: "paddler-tests/never-downloaded".to_owned(),
                revision: "main".to_owned(),
            })
            .into_agent_desired_model(),
        );

        let reconciliation = reconciler
            .reconcile(&cancellation_token, desired_state.clone())
            .await;

        assert_eq!(
            reconciliation,
            DesiredStateReconciliation::Pending(Box::new(desired_state))
        );
        assert_eq!(
            reconciler
                .agent_applicable_state_holder
                .get_agent_applicable_state(),
            AgentApplicableState::default()
        );
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
                agent_desired_state_rx,
                desired_state_reconciler: DesiredStateReconciler {
                    agent_applicable_state_holder: agent_applicable_state_holder.clone(),
                    cache_dir: CacheDir {
                        explicit: Some(cache_dir.path().to_path_buf()),
                        home: None,
                        xdg: None,
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
