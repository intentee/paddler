use serde::Deserialize;
use serde::Serialize;

use crate::agent_decisions_settings::AgentDecisionsSettings;
use crate::agent_desired_model::AgentDesiredModel;
use crate::served_decision_settings::ServedDecisionSettings;

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DecisionSettings {
    pub pointer_head: AgentDesiredModel,
}

impl DecisionSettings {
    #[must_use]
    pub fn agent_decisions(&self) -> AgentDecisionsSettings {
        match &self.pointer_head {
            AgentDesiredModel::None => AgentDecisionsSettings::NotServed,
            AgentDesiredModel::Uri(pointer_head_uri) => {
                AgentDecisionsSettings::Served(ServedDecisionSettings {
                    pointer_head_uri: pointer_head_uri.clone(),
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::DecisionSettings;
    use crate::agent_decisions_settings::AgentDecisionsSettings;
    use crate::agent_desired_model::AgentDesiredModel;
    use crate::served_decision_settings::ServedDecisionSettings;

    #[test]
    fn agents_serve_no_decisions_while_no_pointer_head_is_configured() {
        assert_eq!(
            DecisionSettings::default().agent_decisions(),
            AgentDecisionsSettings::NotServed
        );
    }

    #[test]
    fn agents_serve_decisions_with_the_configured_pointer_head() {
        assert_eq!(
            DecisionSettings {
                pointer_head: AgentDesiredModel::Uri("pointer_head.gguf".to_owned()),
            }
            .agent_decisions(),
            AgentDecisionsSettings::Served(ServedDecisionSettings {
                pointer_head_uri: "pointer_head.gguf".to_owned(),
            })
        );
    }
}
