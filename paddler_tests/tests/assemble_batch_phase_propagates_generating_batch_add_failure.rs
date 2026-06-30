#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use llama_cpp_bindings::SampledToken;
use llama_cpp_bindings::token::LlamaToken;
use paddler_agent::continuous_batch_request_phase::ContinuousBatchRequestPhase;
use paddler_agent::continuous_batch_request_state::ContinuousBatchRequestState;
use paddler_agent::continuous_batch_scheduler::assemble_batch_phase::AssembleBatchPhase;
use paddler_agent::continuous_batch_scheduler::batch_pass::BatchPass;
use paddler_tests::build_active_request::build_active_request;
use paddler_tests::loaded_test_model::LoadedTestModel;

#[test]
fn assemble_batch_phase_propagates_generating_batch_add_failure() -> Result<()> {
    let loaded = LoadedTestModel::qwen3()?;
    let state = ContinuousBatchRequestState {
        current_token_position: 0,
        i_batch: None,
        max_tokens: 64,
        pending_sampled_token: Some(SampledToken::Content(LlamaToken::new(1))),
        phase: ContinuousBatchRequestPhase::Generating,
        prompt_tokens: Vec::new(),
        prompt_tokens_ingested: 0,
        sequence_id: 0,
    };
    let mut requests = [build_active_request(&loaded, state)?];
    let assemble_phase = AssembleBatchPhase { n_batch: 1 };
    let zero_capacity = 0;
    let mut pass = BatchPass::new(zero_capacity, 1)?;

    let result = assemble_phase.run(&mut pass, &mut requests);

    assert!(result.is_err());

    Ok(())
}
