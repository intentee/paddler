use std::sync::Arc;

use log::error;
use tokio_util::sync::CancellationToken;

use paddler_agent_status::slot_aggregated_status::SlotAggregatedStatus;
use paddler_messaging::agent_desired_state::AgentDesiredState;
use paddler_model_source::desired_model_resolver::DesiredModelResolver;

use crate::agent_desired_state_conversion::AgentDesiredStateConversion;
use crate::agent_desired_state_converter::AgentDesiredStateConverter;
use crate::desired_state_reconciliation::DesiredStateReconciliation;

#[derive(Clone)]
pub struct DesiredStateReconciler {
    pub desired_model_resolver: DesiredModelResolver,
    pub slot_aggregated_status: Arc<SlotAggregatedStatus>,
}

impl DesiredStateReconciler {
    pub async fn reconcile(
        &self,
        cancellation_token: &CancellationToken,
        agent_desired_state: AgentDesiredState,
    ) -> DesiredStateReconciliation {
        let conversion = AgentDesiredStateConverter {
            desired_model_resolver: self.desired_model_resolver.clone(),
            cancellation_token: cancellation_token.clone(),
            slot_aggregated_status: self.slot_aggregated_status.clone(),
        }
        .convert(&agent_desired_state)
        .await;

        match conversion {
            Ok(AgentDesiredStateConversion::Converted(applicable_state)) => {
                DesiredStateReconciliation::Reconciled(Box::new(applicable_state))
            }
            Ok(AgentDesiredStateConversion::Cancelled) => {
                DesiredStateReconciliation::Pending(Box::new(agent_desired_state))
            }
            Err(conversion_error) => {
                error!("Failed to convert to applicable state: {conversion_error}");

                DesiredStateReconciliation::Pending(Box::new(agent_desired_state))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::sync::Arc;

    use tokio_util::sync::CancellationToken;

    use paddler_agent_status::slot_aggregated_status::SlotAggregatedStatus;
    use paddler_messaging::agent_desired_model::AgentDesiredModel;
    use paddler_messaging::agent_desired_state::AgentDesiredState;
    use paddler_model_source::agent_local_model_path::AgentLocalModelPath;
    use paddler_model_source::desired_model_resolver::DesiredModelResolver;
    use paddler_model_source::huggingface_default_endpoint::HUGGINGFACE_DEFAULT_ENDPOINT;
    use paddler_model_source::huggingface_model_reference::HuggingFaceModelReference;
    use paddler_model_source::model_source::ModelSource;

    use super::DesiredStateReconciler;
    use crate::desired_state_reconciliation::DesiredStateReconciliation;

    fn desired_state_with_model(model: AgentDesiredModel) -> AgentDesiredState {
        AgentDesiredState {
            model,
            ..AgentDesiredState::default()
        }
    }

    fn desired_state_reconciler() -> DesiredStateReconciler {
        DesiredStateReconciler {
            desired_model_resolver: DesiredModelResolver::from_process_env(
                HUGGINGFACE_DEFAULT_ENDPOINT.to_owned(),
            )
            .unwrap(),
            slot_aggregated_status: Arc::new(SlotAggregatedStatus::new(1)),
        }
    }

    #[tokio::test]
    async fn a_missing_local_model_leaves_the_state_unconverted() {
        let reconciler = desired_state_reconciler();
        let desired_state = desired_state_with_model(
            ModelSource::LocalToAgent(
                AgentLocalModelPath::from_absolute_path(Path::new(
                    "/paddler-nonexistent-model-for-reconciliation.gguf",
                ))
                .unwrap(),
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
    }

    #[tokio::test]
    async fn a_model_download_cancelled_mid_conversion_leaves_the_state_unconverted() {
        let cancellation_token = CancellationToken::new();

        cancellation_token.cancel();

        let reconciler = desired_state_reconciler();
        let desired_state = desired_state_with_model(
            ModelSource::HuggingFace(HuggingFaceModelReference {
                filename: "model.gguf".to_owned(),
                repository_name: "never-downloaded".to_owned(),
                repository_owner: "paddler-tests".to_owned(),
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
    }
}
