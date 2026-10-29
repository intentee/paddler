#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU16;

use tokio::sync::mpsc;

use paddler_agent::pipeline_request::PipelineRequest;
use paddler_agent_runtime::agent_request::AgentRequest;
use paddler_messaging::decision_question::DecisionQuestion;
use paddler_messaging::decision_result::DecisionResult;
use paddler_messaging::request_params::decide_params::DecideParams;
use paddler_test_cluster_harness::model_card::qwen3_0_6b::qwen3_0_6b;

use crate::serving_pipeline_arbiter::ServingPipelineArbiter;

#[tokio::test(flavor = "multi_thread")]
async fn an_agent_without_a_pointer_head_rejects_decisions() {
    let mut serving_pipeline_arbiter = ServingPipelineArbiter::new(1);

    serving_pipeline_arbiter
        .serve(qwen3_0_6b().into_desired_state())
        .await;

    let (response_tx, mut response_rx) = mpsc::unbounded_channel();
    let (_stop_tx, stop_rx) = mpsc::unbounded_channel();

    serving_pipeline_arbiter
        .pipeline_arbiter
        .forward(PipelineRequest::Decide(AgentRequest {
            params: DecideParams {
                last_question: DecisionQuestion {
                    id: "paid".to_owned(),
                    instructions: "Was it paid?".to_owned(),
                    options: vec!["no".to_owned(), "yes".to_owned()],
                },
                leading_questions: Vec::new(),
                state: "The invoice was paid on time.".to_owned(),
            },
            response_tx,
            slot_guard: serving_pipeline_arbiter.slot_guard(NonZeroU16::MIN),
            stop_rx,
        }));

    assert_eq!(
        response_rx.recv().await,
        Some(DecisionResult::DecisionsNotServed(
            "agent: no pointer head is configured, so the agent serves no decisions".to_owned()
        ))
    );

    serving_pipeline_arbiter.shut_down().await;
}
