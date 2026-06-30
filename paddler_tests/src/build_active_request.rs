use std::sync::Arc;

use anyhow::Result;
use llama_cpp_bindings::sampling::LlamaSampler;
use paddler_agent::continuous_batch_active_request::ContinuousBatchActiveRequest;
use paddler_agent::continuous_batch_request_state::ContinuousBatchRequestState;
use paddler_agent::slot_aggregated_status::SlotAggregatedStatus;
use paddler_agent::slot_guard::SlotGuard;
use tokio::sync::mpsc;

use crate::loaded_test_model::LoadedTestModel;

pub fn build_active_request(
    loaded: &LoadedTestModel,
    state: ContinuousBatchRequestState,
) -> Result<ContinuousBatchActiveRequest> {
    let (generated_tokens_tx, _generated_tokens_rx) = mpsc::unbounded_channel();
    let (_generate_tokens_stop_tx, generate_tokens_stop_rx) = mpsc::unbounded_channel();
    let slot_aggregated_status = Arc::new(SlotAggregatedStatus::new(1));

    Ok(ContinuousBatchActiveRequest {
        state,
        chain: LlamaSampler::chain_simple([LlamaSampler::greedy()]),
        token_classifier: loaded.token_classifier()?,
        grammar_sampler: None,
        generated_tokens_tx,
        generate_tokens_stop_rx,
        slot_guard: SlotGuard::new(slot_aggregated_status),
        tool_call_pipeline: None,
    })
}
