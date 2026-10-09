use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use paddler_agent::agent_applicable_state_holder::AgentApplicableStateHolder;
use paddler_agent::agent_error::AgentError;
use paddler_agent::desired_state_reconciler::DesiredStateReconciler;
use paddler_agent::desired_state_reconciliation::DesiredStateReconciliation;
use paddler_agent::pipeline_arbiter_state::PipelineArbiterState;
use paddler_agent_runtime::inference_runtime_context::InferenceRuntimeContext;
use paddler_agent_status::slot_guard::SlotGuard;
use paddler_messaging::agent_desired_state::AgentDesiredState;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;

use crate::agent_inference_runtime_context::agent_inference_runtime_context;

pub struct ServingPipelineArbiter {
    pub desired_state_reconciliation: DesiredStateReconciliation,
    pub inference_runtime_context: InferenceRuntimeContext,
    pub pipeline_arbiter_state: PipelineArbiterState,
}

impl ServingPipelineArbiter {
    pub async fn start(desired_state: BalancerDesiredState, slots: u16) -> Self {
        let agent_applicable_state_holder = Arc::new(AgentApplicableStateHolder::default());
        let inference_runtime_context = agent_inference_runtime_context(slots);
        let desired_state_reconciliation = DesiredStateReconciler {
            agent_applicable_state_holder: agent_applicable_state_holder.clone(),
            slot_aggregated_status: inference_runtime_context.slot_aggregated_status.clone(),
        }
        .reconcile(
            &CancellationToken::new(),
            AgentDesiredState::from(desired_state),
        )
        .await;
        let pipeline_arbiter_state = PipelineArbiterState::Idle
            .apply(
                &CancellationToken::new(),
                agent_applicable_state_holder.get_agent_applicable_state(),
                &inference_runtime_context,
            )
            .await;

        Self {
            desired_state_reconciliation,
            inference_runtime_context,
            pipeline_arbiter_state,
        }
    }

    #[must_use]
    pub fn slot_guard(&self) -> SlotGuard {
        SlotGuard::new(
            self.inference_runtime_context
                .slot_aggregated_status
                .clone(),
        )
    }

    pub async fn shut_down(self) -> Result<(), AgentError> {
        self.pipeline_arbiter_state.shut_down().await
    }
}
