#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use paddler_agent::continuous_batch_request_phase::ContinuousBatchRequestPhase;
use paddler_agent::continuous_batch_request_state::ContinuousBatchRequestState;
use paddler_agent::continuous_batch_scheduler::advance_generating_phase::AdvanceGeneratingPhase;
use paddler_messaging::inference_parameters::InferenceParameters;
use paddler_tests::build_active_request::build_active_request;
use paddler_tests::loaded_test_model::LoadedTestModel;

#[test]
fn advance_generating_phase_skips_generating_request_without_batch_index() -> Result<()> {
    let loaded = LoadedTestModel::qwen3()?;
    let llama_context = loaded.new_context()?;
    let scheduler_context = loaded.scheduler_context(InferenceParameters::default())?;
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

    AdvanceGeneratingPhase {
        scheduler_context: scheduler_context.as_ref(),
        llama_context: &llama_context,
    }
    .run(&mut requests);

    assert!(matches!(
        requests[0].state.phase,
        ContinuousBatchRequestPhase::Generating
    ));

    Ok(())
}
