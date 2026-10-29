use std::collections::BTreeSet;

use paddler_messaging::agent_decisions_serving::AgentDecisionsServing;
use paddler_messaging::agent_issue::AgentIssue;

pub struct ServingReport {
    pub decisions: AgentDecisionsServing,
    pub model_path: String,
    pub serving_issues: BTreeSet<AgentIssue>,
    pub uses_chat_template_override: bool,
}
