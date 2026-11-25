#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use tokio::sync::mpsc;

use paddler_agent::pipeline_request::PipelineRequest;
use paddler_agent_pipeline_harness::serving_pipeline_arbiter::ServingPipelineArbiter;
use paddler_agent_runtime::agent_request::AgentRequest;
use paddler_balancer_desired_state_fixture::decision_desired_state::DecisionDesiredState;
use paddler_messaging::constants::DECISION_SLOTS_MINIMUM;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;

#[tokio::test(flavor = "multi_thread")]
async fn an_agent_serving_a_decision_model_refuses_text_generation() {
    let mut serving_pipeline_arbiter = ServingPipelineArbiter::new(DECISION_SLOTS_MINIMUM.get())
        .expect("the model downloader must start");

    serving_pipeline_arbiter
        .serve(DecisionDesiredState::default().into_balancer_desired_state())
        .await
        .expect("the desired state must be served");

    let (response_tx, mut response_rx) = mpsc::unbounded_channel();
    let (_stop_tx, stop_rx) = mpsc::unbounded_channel();

    serving_pipeline_arbiter
        .pipeline_arbiter
        .forward(PipelineRequest::ContinueFromRawPrompt(AgentRequest {
            params: ContinueFromRawPromptParams {
                grammar: None,
                max_tokens: NonZeroU32::MIN,
                raw_prompt: "Hello".to_owned(),
            },
            response_tx,
            slot_guard: serving_pipeline_arbiter.slot_guard(DECISION_SLOTS_MINIMUM),
            stop_rx,
        }));

    assert!(matches!(
        response_rx.recv().await,
        Some(GeneratedTokenResult::TextGenerationNotServed(_))
    ));

    serving_pipeline_arbiter.shut_down().await;
}
