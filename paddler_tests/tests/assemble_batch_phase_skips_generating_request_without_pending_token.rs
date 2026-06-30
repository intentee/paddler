#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use paddler_agent::continuous_batch_request_phase::ContinuousBatchRequestPhase;
use paddler_agent::continuous_batch_request_state::ContinuousBatchRequestState;
use paddler_agent::continuous_batch_scheduler::assemble_batch_phase::AssembleBatchPhase;
use paddler_agent::continuous_batch_scheduler::batch_pass::BatchPass;
use paddler_tests::build_active_request::build_active_request;
use paddler_tests::loaded_test_model::LoadedTestModel;

#[test]
fn assemble_batch_phase_skips_generating_request_without_pending_token() -> Result<()> {
    let loaded = LoadedTestModel::qwen3()?;
    let state = ContinuousBatchRequestState {
        current_token_position: 0,
        i_batch: None,
        max_tokens: 64,
        pending_sampled_token: None,
        phase: ContinuousBatchRequestPhase::Generating,
        prompt_tokens: Vec::new(),
        prompt_tokens_ingested: 0,
        sequence_id: 0,
    };
    let mut requests = [build_active_request(&loaded, state)?];
    let assemble_phase = AssembleBatchPhase { n_batch: 16 };
    let mut pass = BatchPass::new(16, 1)?;

    assemble_phase.run(&mut pass, &mut requests)?;

    assert!(pass.contributions.generating.is_empty());

    Ok(())
}
