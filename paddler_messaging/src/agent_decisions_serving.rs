use serde::Deserialize;
use serde::Serialize;

use crate::inference_capability::InferenceCapability;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub enum AgentDecisionsServing {
    DecisionModelNotServed,
    NotServed,
    Served,
}

impl AgentDecisionsServing {
    #[must_use]
    pub const fn serves(self, inference_capability: InferenceCapability) -> bool {
        match inference_capability {
            InferenceCapability::Decisions => match self {
                Self::DecisionModelNotServed | Self::NotServed => false,
                Self::Served => true,
            },
            InferenceCapability::Embeddings => match self {
                Self::DecisionModelNotServed | Self::Served => false,
                Self::NotServed => true,
            },
            InferenceCapability::TextGeneration => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::AgentDecisionsServing;
    use crate::inference_capability::InferenceCapability;

    #[test]
    fn only_an_agent_serving_decisions_serves_decisions() {
        assert!(
            !AgentDecisionsServing::DecisionModelNotServed.serves(InferenceCapability::Decisions)
        );
        assert!(!AgentDecisionsServing::NotServed.serves(InferenceCapability::Decisions));
        assert!(AgentDecisionsServing::Served.serves(InferenceCapability::Decisions));
    }

    #[test]
    fn only_an_agent_serving_a_model_that_makes_no_decisions_serves_embeddings() {
        assert!(
            !AgentDecisionsServing::DecisionModelNotServed.serves(InferenceCapability::Embeddings)
        );
        assert!(AgentDecisionsServing::NotServed.serves(InferenceCapability::Embeddings));
        assert!(!AgentDecisionsServing::Served.serves(InferenceCapability::Embeddings));
    }

    #[test]
    fn every_serving_agent_serves_text_generation() {
        for agent_decisions_serving in [
            AgentDecisionsServing::DecisionModelNotServed,
            AgentDecisionsServing::NotServed,
            AgentDecisionsServing::Served,
        ] {
            assert!(agent_decisions_serving.serves(InferenceCapability::TextGeneration));
        }
    }
}
