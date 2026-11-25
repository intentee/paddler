#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU16;

use llama_cpp_context::context_error::ContextError;
use llama_cpp_context::llama_batch::LlamaBatch;

use paddler_agent_decision::decision_decoder::DecisionDecoder;
use paddler_agent_decision::decision_error::DecisionError;
use paddler_agent_decision::decision_prompt_cursor::DecisionPromptCursor;
use paddler_agent_decision::decision_prompt_outputs_request::DecisionPromptOutputsRequest;
use paddler_agent_decision_tests::decision_model_runtime::DecisionModelRuntime;
use paddler_agent_decision_tests::snowy_weather_decision::snowy_weather_decision;
use paddler_agent_runtime::sequence_id::SequenceId;
use paddler_inference_parameters::batch_size::BatchSize;
use paddler_inference_parameters::model_default_or::ModelDefaultOr;
use paddler_model_card_fixture::kev_0_8b::kev_0_8b;

const BATCH_SHORTER_THAN_THE_PROMPT: u32 = 16;

#[test]
fn a_decision_decoding_more_tokens_than_its_batch_holds_fails_as_batch_assembly() {
    let mut decision_model_runtime =
        DecisionModelRuntime::load(&kev_0_8b(), 1).expect("the kev decision model must load");

    decision_model_runtime
        .runtime
        .model_runtime_parameters
        .n_batch = ModelDefaultOr::Explicit(
        BatchSize::try_from(BATCH_SHORTER_THAN_THE_PROMPT).expect("the batch size is valid"),
    );

    let prepared_decision = decision_model_runtime
        .prepared_decision(snowy_weather_decision(), NonZeroU16::MIN, u32::MAX)
        .expect("the decision must be prepared");
    let prompt = prepared_decision
        .request
        .plan
        .prompts
        .first()
        .expect("the decision must have a prompt");
    let mut llama_context = decision_model_runtime
        .create_llama_context(1)
        .expect("the decision context must be created");
    let mut batch = LlamaBatch::new(llama_context.limits());

    assert!(matches!(
        DecisionDecoder {
            batch: &mut batch,
            llama_context: &mut llama_context,
        }
        .decode_text(
            prompt,
            SequenceId(0),
            DecisionPromptCursor {
                llama_position: 0,
                token_index: 0,
            },
            0..prompt.tokens().len(),
            DecisionPromptOutputsRequest::None,
        ),
        Err(DecisionError::BatchAssemblyFailed(
            ContextError::BatchFull { .. }
        ))
    ));
}
