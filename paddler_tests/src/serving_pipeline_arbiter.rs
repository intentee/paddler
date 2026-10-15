use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use paddler_agent::agent_applicable_state_holder::AgentApplicableStateHolder;
use paddler_agent::desired_state_reconciler::DesiredStateReconciler;
use paddler_agent::desired_state_reconciliation::DesiredStateReconciliation;
use paddler_agent::pipeline_arbiter::PipelineArbiter;
use paddler_agent::pipeline_arbiter_params::PipelineArbiterParams;
use paddler_agent_runtime::inference_runtime_context::InferenceRuntimeContext;
use paddler_agent_status::slot_guard::SlotGuard;
use paddler_cache_dir::cache_dir::CacheDir;
use paddler_messaging::agent_desired_state::AgentDesiredState;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;

use crate::agent_inference_runtime_context::agent_inference_runtime_context;

pub struct ServingPipelineArbiter {
    pub desired_state_reconciliation: DesiredStateReconciliation,
    pub inference_runtime_context: InferenceRuntimeContext,
    pub pipeline_arbiter: PipelineArbiter,
}

impl ServingPipelineArbiter {
    pub async fn start(desired_state: BalancerDesiredState, slots: u16) -> Self {
        let agent_applicable_state_holder = Arc::new(AgentApplicableStateHolder::default());
        let inference_runtime_context = agent_inference_runtime_context(slots);
        let desired_state_reconciliation = DesiredStateReconciler {
            agent_applicable_state_holder: agent_applicable_state_holder.clone(),
            cache_dir: CacheDir::from_process_env(),
            slot_aggregated_status: inference_runtime_context.slot_aggregated_status.clone(),
        }
        .reconcile(
            &CancellationToken::new(),
            AgentDesiredState::from(desired_state),
        )
        .await;
        let mut pipeline_arbiter = PipelineArbiter::new(PipelineArbiterParams {
            agent_shutdown: CancellationToken::new(),
            inference_runtime_context: inference_runtime_context.clone(),
            served_applicable_state_holder: Arc::new(AgentApplicableStateHolder::default()),
        });

        pipeline_arbiter.apply(agent_applicable_state_holder.get_agent_applicable_state());

        let finished_pipeline_startup = pipeline_arbiter.next_finished_startup().await;

        pipeline_arbiter.conclude(finished_pipeline_startup);

        Self {
            desired_state_reconciliation,
            inference_runtime_context,
            pipeline_arbiter,
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

    pub async fn shut_down(self) {
        self.pipeline_arbiter.shut_down().await;
    }
}
