#![cfg(feature = "tests_that_use_llms")]

use llama_cpp_sampler::sampler_error::SamplerError;

use paddler_agent_pipeline_harness::raw_prompt_generation::RawPromptGeneration;
use paddler_agent_text_generation::text_generation_error::TextGenerationError;
use paddler_agent_text_generation_tests::generation_preparation_without_memory_for_the_penalty_window::GenerationPreparationWithoutMemoryForThePenaltyWindow;

#[test]
fn a_sampler_chain_that_cannot_allocate_its_penalty_window_fails_the_raw_prompt_preparation() {
    let preparation = GenerationPreparationWithoutMemoryForThePenaltyWindow::load()
        .expect("the generation preparation must load");
    let RawPromptGeneration { agent_request, .. } = RawPromptGeneration::new(
        "Paddler",
        preparation
            .llama_runtime
            .inference_runtime_context
            .slot_aggregated_status
            .clone(),
    );

    GenerationPreparationWithoutMemoryForThePenaltyWindow::refuse_the_penalty_window_allocation()
        .expect("the penalty window allocation must be refused");

    assert!(matches!(
        preparation
            .request_preparer
            .prepare_raw_prompt(agent_request),
        Err(TextGenerationError::SamplerChainCreationFailed(
            SamplerError::InitLlamaCppOutOfMemory {
                sampler: "penalties"
            }
        ))
    ));
}
