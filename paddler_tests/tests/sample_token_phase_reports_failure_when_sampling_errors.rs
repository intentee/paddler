#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use paddler_agent::continuous_batch_request_phase::ContinuousBatchRequestPhase;
use paddler_agent::continuous_batch_request_state::ContinuousBatchRequestState;
use paddler_agent::continuous_batch_scheduler::sample_outcome::SampleOutcome;
use paddler_agent::continuous_batch_scheduler::sample_token_phase::SampleTokenPhase;
use paddler_tests::build_active_request::build_active_request;
use paddler_tests::loaded_test_model::LoadedTestModel;

#[test]
fn sample_token_phase_reports_failure_when_sampling_errors() -> Result<()> {
    let loaded = LoadedTestModel::qwen3()?;
    let context = loaded.new_context()?;
    let state = ContinuousBatchRequestState {
        current_token_position: 0,
        i_batch: Some(0),
        max_tokens: 64,
        pending_sampled_token: None,
        phase: ContinuousBatchRequestPhase::Generating,
        prompt_tokens: Vec::new(),
        prompt_tokens_ingested: 0,
        sequence_id: 0,
    };
    let mut request = build_active_request(&loaded, state)?;

    let outcome = SampleTokenPhase { context: &context }.run(&mut request, 0);

    assert!(matches!(outcome, SampleOutcome::Failed(_)));

    Ok(())
}
