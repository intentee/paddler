#![cfg(feature = "tests_that_use_llms")]

use tokio::sync::mpsc;
use tokio::sync::mpsc::UnboundedReceiver;

use paddler_agent::pipeline_request::PipelineRequest;
use paddler_agent_runtime::agent_request::AgentRequest;
use paddler_messaging::decision_result::DecisionResult;
use paddler_messaging::decision_slots_minimum::DECISION_SLOTS_MINIMUM;
use paddler_messaging::request_params::decide_params::raw_decide_params::RawDecideParams;
use paddler_messaging::validates::Validates as _;
use paddler_test_cluster_harness::model_card::qwen3_5_0_8b::qwen3_5_0_8b;
use paddler_test_cluster_harness::pointer_head_fixture::pointer_head_fixture;
use paddler_test_cluster_harness::synthetic_pointer_head_fixture::QWEN3_5_0_8B_SYNTHETIC_POINTER_HEAD_FIXTURE;

use crate::serving_pipeline_arbiter::ServingPipelineArbiter;

pub struct ServingDecisionAgent {
    pub serving_pipeline_arbiter: ServingPipelineArbiter,
}

impl ServingDecisionAgent {
    pub async fn start() -> Self {
        let mut serving_pipeline_arbiter =
            ServingPipelineArbiter::new(DECISION_SLOTS_MINIMUM.get());

        serving_pipeline_arbiter
            .serve(
                qwen3_5_0_8b().into_decision_desired_state(
                    pointer_head_fixture(QWEN3_5_0_8B_SYNTHETIC_POINTER_HEAD_FIXTURE)
                        .expect("the pointer head fixture path must be absolute"),
                ),
            )
            .await;

        Self {
            serving_pipeline_arbiter,
        }
    }

    pub fn forward_decision(
        &self,
        raw_decide_params: RawDecideParams,
        decision_stop_rx: UnboundedReceiver<()>,
    ) -> UnboundedReceiver<DecisionResult> {
        let (decision_result_tx, decision_result_rx) = mpsc::unbounded_channel();
        let decide_params = raw_decide_params
            .validate()
            .expect("the decision parameters must be valid");

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
    }

    pub async fn shut_down(self) {
        self.serving_pipeline_arbiter.shut_down().await;
    }
}
