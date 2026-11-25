use std::num::NonZeroU16;

use paddler_agent_runtime::inference_runtime_context::InferenceRuntimeContext;
use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::agent_issue_params::slots_insufficient_for_decisions_params::SlotsInsufficientForDecisionsParams;
use paddler_messaging::constants::DECISION_SLOTS_MINIMUM;

use crate::decision_error::DecisionError;

pub struct DecisionSlots;

impl DecisionSlots {
    /// # Errors
    ///
    /// Returns [`DecisionError::SlotsInsufficient`] when the agent has fewer slots than a decision with a question lane needs, registering the issue.
    pub fn allot(
        InferenceRuntimeContext {
            slot_aggregated_status,
            ..
        }: &InferenceRuntimeContext,
    ) -> Result<NonZeroU16, DecisionError> {
        let desired_slots = slot_aggregated_status.desired_slots_total;

        NonZeroU16::new(desired_slots)
            .filter(|decision_slots| *decision_slots >= DECISION_SLOTS_MINIMUM)
            .ok_or_else(|| {
                slot_aggregated_status.register_pipeline_issue(
                    AgentIssue::SlotsInsufficientForDecisions(
                        SlotsInsufficientForDecisionsParams {
                            desired_slots,
                            required_slots: DECISION_SLOTS_MINIMUM.get(),
                        },
                    ),
                );

                DecisionError::SlotsInsufficient {
                    desired_slots,
                    required_slots: DECISION_SLOTS_MINIMUM.get(),
                }
            })
    }
}
