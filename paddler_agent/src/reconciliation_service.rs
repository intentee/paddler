use anyhow::Result;
use async_trait::async_trait;
use log::error;
use tokio::select;
use tokio::sync::mpsc;
use tokio::time::Duration;
use tokio::time::MissedTickBehavior;
use tokio::time::interval;
use tokio_util::sync::CancellationToken;
use trzcina::Service;

use paddler_messaging::agent_desired_state::AgentDesiredState;

use crate::desired_state_reconciler::DesiredStateReconciler;
use crate::desired_state_reconciliation::DesiredStateReconciliation;

const DESIRED_STATE_CONVERSION_RETRY_INTERVAL: Duration = Duration::from_secs(1);

pub struct ReconciliationService {
    pub agent_desired_state_rx: mpsc::UnboundedReceiver<AgentDesiredState>,
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

        let mut reconciliation = DesiredStateReconciliation::Reconciled;
        let mut ticker = interval(DESIRED_STATE_CONVERSION_RETRY_INTERVAL);

        ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);

        loop {
            select! {
                biased;
                () = shutdown.cancelled() => break Ok(()),
                _ = ticker.tick() => {
                    if let DesiredStateReconciliation::Pending(agent_desired_state) = reconciliation {
                        reconciliation = desired_state_reconciler
                            .reconcile(&shutdown, *agent_desired_state)
                            .await;
                    }
                },
                next_agent_desired_state = agent_desired_state_rx.recv() => {
                    let Some(agent_desired_state) = next_agent_desired_state else {
                        error!("Agent desired state channel closed, stopping reconciliation service.");

                        break Ok(());
                    };

                    reconciliation = desired_state_reconciler
                        .reconcile(&shutdown, agent_desired_state)
                        .await;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use tokio_util::sync::CancellationToken;

    use paddler_agent_status::slot_aggregated_status::SlotAggregatedStatus;
    use paddler_inference_parameters::inference_parameters::InferenceParameters;
    use paddler_messaging::agent_desired_model::AgentDesiredModel;
    use paddler_messaging::agent_desired_state::AgentDesiredState;
    use paddler_messaging::huggingface_model_reference::HuggingFaceModelReference;

    use crate::agent_applicable_state::AgentApplicableState;
    use crate::agent_applicable_state_holder::AgentApplicableStateHolder;
    use crate::desired_state_reconciler::DesiredStateReconciler;
    use crate::desired_state_reconciliation::DesiredStateReconciliation;

    fn desired_state_with_model(model: AgentDesiredModel) -> AgentDesiredState {
        AgentDesiredState {
            chat_template_override: None,
            inference_parameters: InferenceParameters::default(),
            model,
            multimodal_projection: AgentDesiredModel::None,
        }
    }

    fn desired_state_reconciler() -> DesiredStateReconciler {
        DesiredStateReconciler {
            agent_applicable_state_holder: Arc::new(AgentApplicableStateHolder::default()),
            slot_aggregated_status: Arc::new(SlotAggregatedStatus::new(1)),
        }
    }

    #[tokio::test]
    async fn a_missing_local_model_leaves_the_state_unconverted() {
        let reconciler = desired_state_reconciler();
        let desired_state = desired_state_with_model(AgentDesiredModel::LocalToAgent(
            "/paddler-nonexistent-model-for-reconciliation.gguf".to_owned(),
        ));

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
        let desired_state =
            desired_state_with_model(AgentDesiredModel::HuggingFace(HuggingFaceModelReference {
                filename: "model.gguf".to_owned(),
                repo_id: "paddler-tests/never-downloaded".to_owned(),
                revision: "main".to_owned(),
            }));

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
}
