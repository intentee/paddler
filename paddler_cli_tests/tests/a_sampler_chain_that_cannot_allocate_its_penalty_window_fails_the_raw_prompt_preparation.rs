#![cfg(feature = "tests_that_use_llms")]

use paddler_tests::raw_prompt_generation::RawPromptGeneration;

use crate::generation_preparation_without_memory_for_the_penalty_window::GenerationPreparationWithoutMemoryForThePenaltyWindow;

#[test]
fn a_sampler_chain_that_cannot_allocate_its_penalty_window_fails_the_raw_prompt_preparation() {
    let preparation = GenerationPreparationWithoutMemoryForThePenaltyWindow::load();
    let RawPromptGeneration { agent_request, .. } = RawPromptGeneration::new(
        "Paddler",
        preparation
            .llama_runtime
            .inference_runtime_context
            .slot_aggregated_status
            .clone(),
    );

    GenerationPreparationWithoutMemoryForThePenaltyWindow::refuse_the_penalty_window_allocation();

    assert_eq!(
        preparation
            .request_preparer
            .prepare_raw_prompt(agent_request)
            .err()
            .as_ref()
            .map(ToString::to_string),
        Some(GenerationPreparationWithoutMemoryForThePenaltyWindow::penalty_window_allocation_failure())
    );
}
