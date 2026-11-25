use std::num::NonZeroU16;

use paddler_agent_runtime::inference_runtime_context::InferenceRuntimeContext;
use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::agent_issue_params::slots_insufficient_for_decisions_params::SlotsInsufficientForDecisionsParams;
use paddler_messaging::decision_slot_allotment::DecisionSlotAllotment;

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
        match DecisionSlotAllotment::of_desired_slots(slot_aggregated_status.desired_slots_total) {
            DecisionSlotAllotment::Allotted(decision_slots) => Ok(decision_slots),
            DecisionSlotAllotment::Insufficient(slots_insufficient_for_decisions_params) => {
                let SlotsInsufficientForDecisionsParams {
                    desired_slots,
                    required_slots,
                } = slots_insufficient_for_decisions_params;

                slot_aggregated_status.register_pipeline_issue(
                    AgentIssue::SlotsInsufficientForDecisions(
                        SlotsInsufficientForDecisionsParams {
                            desired_slots,
                            required_slots,
                        },
                    ),
                );

                Err(DecisionError::SlotsInsufficient {
                    desired_slots,
                    required_slots,
                })
            }
        }
    }
}
