#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use llama_cpp_context::context_error::ContextError;

use paddler_agent_decision::decision_error::DecisionError;
use paddler_agent_decision_tests::decision_in_a_small_kv_cache::DecisionInASmallKvCache;

const CACHE_CELLS: NonZeroU32 = NonZeroU32::new(256).unwrap();
const STATE_REPETITIONS_OVERFLOWING_THE_CACHE: usize = 20;
const QUESTIONS_SHARING_THEIR_PROMPT: usize = 2;

#[test]
fn a_shared_decision_prefix_the_kv_cache_cannot_hold_fails_the_decision() {
    assert!(matches!(
        DecisionInASmallKvCache {
            cache_cells: CACHE_CELLS,
            question_count: QUESTIONS_SHARING_THEIR_PROMPT,
            state_repetitions: STATE_REPETITIONS_OVERFLOWING_THE_CACHE,
        }
        .final_advance()
        .expect("the decision must be admitted to the kv cache"),
        Err(DecisionError::DecodeFailed(
            ContextError::DecodeNoKvCacheSlot
        ))
    ));
}
