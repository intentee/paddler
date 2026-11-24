use std::num::NonZeroU16;

use paddler_agent_runtime::inference_runtime_context::InferenceRuntimeContext;
use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::agent_issue_params::slots_insufficient_for_decisions_params::SlotsInsufficientForDecisionsParams;
use paddler_messaging::constants::DECISION_SLOTS_MINIMUM;

pub enum DecisionSlots {
    Insufficient,
    Sufficient(NonZeroU16),
}

impl DecisionSlots {
    #[must_use]
    pub fn allot(
        InferenceRuntimeContext {
            slot_aggregated_status,
            ..
        }: &InferenceRuntimeContext,
    ) -> Self {
        let desired_slots = slot_aggregated_status.desired_slots_total;

        NonZeroU16::new(desired_slots)
            .filter(|decision_slots| *decision_slots >= DECISION_SLOTS_MINIMUM)
            .map_or_else(
                || {
                    slot_aggregated_status.register_pipeline_issue(
                        AgentIssue::SlotsInsufficientForDecisions(
                            SlotsInsufficientForDecisionsParams {
                                desired_slots,
                                required_slots: DECISION_SLOTS_MINIMUM.get(),
                            },
                        ),
                    );

                    Self::Insufficient
                },
                Self::Sufficient,
            )
    }
}
