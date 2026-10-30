use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::agent_issue_params::slots_insufficient_for_decisions_params::SlotsInsufficientForDecisionsParams;
use paddler_messaging::decision_slots_minimum::DECISION_SLOTS_MINIMUM;

pub fn slots_insufficient_for_decisions(desired_slots: u16) -> AgentIssue {
    AgentIssue::SlotsInsufficientForDecisions(SlotsInsufficientForDecisionsParams {
        desired_slots,
        required_slots: DECISION_SLOTS_MINIMUM.get(),
    })
}
