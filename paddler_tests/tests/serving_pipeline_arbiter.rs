#[cfg(feature = "tests_that_use_llms")]
use std::mem::discriminant;
#[cfg(feature = "tests_that_use_llms")]
use std::num::NonZeroU16;
use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use paddler_agent::agent_applicable_state_holder::AgentApplicableStateHolder;
use paddler_agent::desired_state_reconciler::DesiredStateReconciler;
use paddler_agent::desired_state_reconciliation::DesiredStateReconciliation;
use paddler_agent::pipeline_arbiter::PipelineArbiter;
use paddler_agent::pipeline_arbiter_params::PipelineArbiterParams;
use paddler_agent_runtime::inference_runtime_context::InferenceRuntimeContext;
#[cfg(feature = "tests_that_use_llms")]
use paddler_agent_status::slot_guard::SlotGuard;
use paddler_messaging::agent_desired_state::AgentDesiredState;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_model_source::desired_model_resolver::DesiredModelResolver;
use paddler_model_source::huggingface_default_endpoint::HUGGINGFACE_DEFAULT_ENDPOINT;
use paddler_tests::agent_inference_runtime_context::agent_inference_runtime_context;
#[cfg(feature = "tests_that_use_llms")]
use paddler_tests::desired_state_reconciled_discriminant::desired_state_reconciled_discriminant;

pub struct ServingPipelineArbiter {
    pub desired_state_reconciler: DesiredStateReconciler,
    pub inference_runtime_context: InferenceRuntimeContext,
    pub pipeline_arbiter: PipelineArbiter,
}

impl ServingPipelineArbiter {
    #[must_use]
    pub fn new(slots: u16) -> Self {
        let inference_runtime_context = agent_inference_runtime_context(slots);

        Self {
            desired_state_reconciler: DesiredStateReconciler {
                desired_model_resolver: DesiredModelResolver::from_process_env(
                    HUGGINGFACE_DEFAULT_ENDPOINT.to_owned(),
                )
                .expect("the model downloader must start"),
                slot_aggregated_status: inference_runtime_context.slot_aggregated_status.clone(),
            },
            pipeline_arbiter: PipelineArbiter::new(PipelineArbiterParams {
                agent_shutdown: CancellationToken::new(),
                inference_runtime_context: inference_runtime_context.clone(),
                served_applicable_state_holder: Arc::new(AgentApplicableStateHolder::default()),
            }),
            inference_runtime_context,
        }
    }

    #[cfg(feature = "tests_that_use_llms")]
    pub async fn serve(&mut self, desired_state: BalancerDesiredState) {
        let desired_state_reconciliation = self.apply(desired_state).await;

        assert_eq!(
            discriminant(&desired_state_reconciliation),
            desired_state_reconciled_discriminant()
        );

        self.conclude_next_progress().await;
        self.conclude_next_progress().await;
    }

    #[cfg(feature = "tests_that_use_llms")]
    pub async fn conclude_next_progress(&mut self) {
        let serving_startup_progress = self.pipeline_arbiter.next_finished_startup().await;

        self.pipeline_arbiter.conclude(serving_startup_progress);
    }

    pub async fn apply(
        &mut self,
        desired_state: BalancerDesiredState,
    ) -> DesiredStateReconciliation {
        let desired_state_reconciliation = self
            .desired_state_reconciler
            .reconcile(
                &CancellationToken::new(),
                AgentDesiredState::from(desired_state),
            )
            .await;

        if let DesiredStateReconciliation::Reconciled(agent_applicable_state) =
            &desired_state_reconciliation
        {
            self.pipeline_arbiter.apply(*agent_applicable_state.clone());
        }

        desired_state_reconciliation
    }

    #[cfg(feature = "tests_that_use_llms")]
    #[must_use]
    pub fn slot_guard(&self, reserved_slots: NonZeroU16) -> SlotGuard {
        SlotGuard::new(
            self.inference_runtime_context
                .slot_aggregated_status
                .clone(),
            reserved_slots,
        )
    }

    pub async fn shut_down(self) {
        self.pipeline_arbiter.shut_down().await;
    }
}
