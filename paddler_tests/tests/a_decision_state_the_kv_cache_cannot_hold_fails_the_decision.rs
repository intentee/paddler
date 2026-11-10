#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use llama_cpp_bindings::error::decode_error::DecodeError;

use paddler_agent_decision::decision_advance::DecisionAdvance;
use paddler_agent_decision::decision_error::DecisionError;
use paddler_tests::kev_decision_in_a_kv_cache::KevDecisionInAKvCache;

const CACHE_CELLS: NonZeroU32 = NonZeroU32::new(256).unwrap();
const STATE_TOKENS_OVERFLOWING_THE_CACHE_BESIDE_THE_DELIMITER: usize = 256;

#[test]
fn a_decision_state_the_kv_cache_cannot_hold_fails_the_decision() {
    assert_eq!(
        KevDecisionInAKvCache {
            cache_cells: CACHE_CELLS,
            state_tokens_after_the_delimiter:
                STATE_TOKENS_OVERFLOWING_THE_CACHE_BESIDE_THE_DELIMITER,
        }
        .first_two_advances()
        .expect("the decision must be admitted to the kv cache"),
        [
            Ok(DecisionAdvance::Continuing),
            Err(DecisionError::DecodeFailed(DecodeError::NoKvCacheSlot).to_string()),
        ]
    );
}
