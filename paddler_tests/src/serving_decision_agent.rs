use anyhow::Context as _;
use anyhow::Result;
use futures_util::TryFutureExt as _;
use futures_util::future::ready;
use tokio::sync::mpsc;
use tokio::sync::mpsc::UnboundedReceiver;

use paddler_agent::pipeline_request::PipelineRequest;
use paddler_agent_runtime::agent_request::AgentRequest;
use paddler_messaging::constants::DECISION_SLOTS_MINIMUM;
use paddler_messaging::decision_result::DecisionResult;
use paddler_messaging::request_params::decide_params::raw_decide_params::RawDecideParams;
use paddler_messaging::request_params_validation_error::RequestParamsValidationError;
use paddler_messaging::validates::Validates as _;
use paddler_test_cluster_harness::constants::QWEN3_5_0_8B_SYNTHETIC_POINTER_HEAD_FIXTURE;
use paddler_test_cluster_harness::model_card::qwen3_5_0_8b::qwen3_5_0_8b;
use paddler_test_cluster_harness::pointer_head_fixture::pointer_head_fixture;

use crate::serving_pipeline_arbiter::ServingPipelineArbiter;

pub struct ServingDecisionAgent {
    pub serving_pipeline_arbiter: ServingPipelineArbiter,
}

impl ServingDecisionAgent {
    /// # Errors
    ///
    /// Returns an error when the model downloader cannot start, when the pointer head fixture lies at a path that cannot be served, and when the decision desired state is not reconciled.
    pub async fn start() -> Result<Self> {
        ready(
            ServingPipelineArbiter::new(DECISION_SLOTS_MINIMUM.get())
                .context("the serving pipeline arbiter must start"),
        )
        .and_then(Self::serving_the_decision_model)
        .await
    }

    async fn serving_the_decision_model(
        mut serving_pipeline_arbiter: ServingPipelineArbiter,
    ) -> Result<Self> {
        ready(
            pointer_head_fixture(QWEN3_5_0_8B_SYNTHETIC_POINTER_HEAD_FIXTURE)
                .context("the pointer head fixture path must be absolute"),
        )
        .and_then(|pointer_head| {
            serving_pipeline_arbiter.serve(qwen3_5_0_8b().into_decision_desired_state(pointer_head))
        })
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
        raw_decide_params: RawDecideParams,
        decision_stop_rx: UnboundedReceiver<()>,
    ) -> Result<UnboundedReceiver<DecisionResult>, RequestParamsValidationError> {
        raw_decide_params.validate().map(|decide_params| {
            let (decision_result_tx, decision_result_rx) = mpsc::unbounded_channel();

            self.serving_pipeline_arbiter
                .pipeline_arbiter
                .forward(PipelineRequest::Decide(AgentRequest {
                    slot_guard: self
                        .serving_pipeline_arbiter
                        .slot_guard(decide_params.required_slots()),
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
