use std::mem::Discriminant;
use std::mem::discriminant;

use paddler_messaging::decision_result::DecisionResult;
use paddler_messaging::decision_summary::DecisionSummary;

#[must_use]
pub const fn decision_done_discriminant() -> Discriminant<DecisionResult> {
    discriminant(&DecisionResult::Done(DecisionSummary {
        input_tokens: 0,
        processing_milliseconds: 0,
    }))
}
