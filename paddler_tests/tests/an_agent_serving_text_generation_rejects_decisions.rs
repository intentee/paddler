#![cfg(feature = "tests_that_use_llms")]

use tokio::sync::mpsc;

use paddler_agent::desired_state_reconciliation::DesiredStateReconciliation;
use paddler_agent::pipeline_request::PipelineRequest;
use paddler_agent_runtime::agent_request::AgentRequest;
use paddler_messaging::decision_question::DecisionQuestion;
use paddler_messaging::decision_result::DecisionResult;
use paddler_messaging::request_params::decide_params::DecideParams;
use paddler_test_cluster_harness::model_card::qwen3_0_6b::qwen3_0_6b;
use paddler_tests::serving_pipeline_arbiter::ServingPipelineArbiter;

#[tokio::test(flavor = "multi_thread")]
async fn an_agent_serving_text_generation_rejects_decisions() {
    let serving_pipeline_arbiter =
        ServingPipelineArbiter::start(qwen3_0_6b().into_desired_state(), 1).await;

    assert_eq!(
        serving_pipeline_arbiter.desired_state_reconciliation,
        DesiredStateReconciliation::Reconciled
    );

    let (response_tx, mut response_rx) = mpsc::unbounded_channel();
    let (_stop_tx, stop_rx) = mpsc::unbounded_channel();

    serving_pipeline_arbiter.pipeline_arbiter_state.forward(
        serving_pipeline_arbiter
            .inference_runtime_context
            .agent_name
            .as_deref(),
        PipelineRequest::Decide(AgentRequest {
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
            slot_guard: serving_pipeline_arbiter.slot_guard(),
            stop_rx,
        }),
    );

    assert_eq!(
        response_rx.recv().await,
        Some(DecisionResult::InferenceModeMismatch(
            "agent: the agent serves TextGeneration, so it cannot decide".to_owned()
        ))
    );

    serving_pipeline_arbiter
        .shut_down()
        .await
        .expect("the agent must shut down cleanly");
}
