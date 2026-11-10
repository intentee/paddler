use paddler_agent_decision::met_decision_prerequisites::MetDecisionPrerequisites;

pub enum PreparedDecisions {
    Disabled,
    Enabled(Box<MetDecisionPrerequisites>),
}
