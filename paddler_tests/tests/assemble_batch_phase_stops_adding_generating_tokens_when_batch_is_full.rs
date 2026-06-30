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

const fn generating_state_with_pending_token(sequence_id: u16) -> ContinuousBatchRequestState {
    ContinuousBatchRequestState {
        current_token_position: 0,
        i_batch: None,
        max_tokens: 64,
        pending_sampled_token: Some(SampledToken::Content(LlamaToken::new(1))),
        phase: ContinuousBatchRequestPhase::Generating,
        prompt_tokens: Vec::new(),
        prompt_tokens_ingested: 0,
        sequence_id,
    }
}

#[test]
fn assemble_batch_phase_stops_adding_generating_tokens_when_batch_is_full() -> Result<()> {
    let loaded = LoadedTestModel::qwen3()?;
    let mut requests = [
        build_active_request(&loaded, generating_state_with_pending_token(0))?,
        build_active_request(&loaded, generating_state_with_pending_token(1))?,
    ];
    let assemble_phase = AssembleBatchPhase { n_batch: 1 };
    let mut pass = BatchPass::new(1, 2)?;

    assemble_phase.run(&mut pass, &mut requests)?;

    assert_eq!(pass.contributions.generating.len(), 1);

    Ok(())
}
