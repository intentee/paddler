use anyhow::Context as _;
use anyhow::Result;
use futures_util::TryFutureExt as _;
use futures_util::future::ready;
use tokio::sync::mpsc;
use tokio::sync::mpsc::UnboundedReceiver;

use llama_cpp_bindings_types::decision_request::DecisionRequest;

use paddler_agent::pipeline_request::PipelineRequest;
use paddler_agent_pipeline_harness::serving_pipeline_arbiter::ServingPipelineArbiter;
use paddler_agent_runtime::agent_request::AgentRequest;
use paddler_messaging::constants::DECISION_SLOTS_WITH_A_QUESTION_LANE;
use paddler_messaging::decision_result::DecisionResult;
use paddler_messaging::request_params::validates::Validates as _;
use paddler_messaging::request_params_validation_error::RequestParamsValidationError;
use paddler_model_card_fixture::kev_0_8b::kev_0_8b;

pub struct ServingDecisionAgent {
    pub serving_pipeline_arbiter: ServingPipelineArbiter,
}

impl ServingDecisionAgent {
    /// # Errors
    ///
    /// Returns an error when the model downloader cannot start, and when the kev desired state is not reconciled.
    pub async fn start() -> Result<Self> {
        ready(
            ServingPipelineArbiter::new(DECISION_SLOTS_WITH_A_QUESTION_LANE.get())
                .context("the serving pipeline arbiter must start"),
        )
        .and_then(Self::serving_the_decision_model)
        .await
    }

    async fn serving_the_decision_model(
        mut serving_pipeline_arbiter: ServingPipelineArbiter,
    ) -> Result<Self> {
        serving_pipeline_arbiter
            .serve(kev_0_8b().into_desired_state())
            .await
            .map(|()| Self {
                serving_pipeline_arbiter,
            })
    }

    /// # Errors
    ///
    /// Returns the [`RequestParamsValidationError`] of decision parameters that are not valid.
    pub fn forward_decision(
        &self,
        decision_request: DecisionRequest,
        decision_stop_rx: UnboundedReceiver<()>,
    ) -> Result<UnboundedReceiver<DecisionResult>, RequestParamsValidationError> {
        decision_request.validate().map(|decide_params| {
            let (decision_result_tx, decision_result_rx) = mpsc::unbounded_channel();

            self.serving_pipeline_arbiter
                .pipeline_arbiter
                .forward(PipelineRequest::Decide(AgentRequest {
                    slot_guard: self
                        .serving_pipeline_arbiter
                        .slot_guard(DECISION_SLOTS_WITH_A_QUESTION_LANE),
                    params: decide_params,
                    response_tx: decision_result_tx,
                    stop_rx: decision_stop_rx,
                }));

            decision_result_rx
        })
    }

    pub async fn shut_down(self) {
        self.serving_pipeline_arbiter.shut_down().await;
    }
}
