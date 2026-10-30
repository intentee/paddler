use serde::Deserialize;
use serde::Serialize;

use crate::inference_capability::InferenceCapability;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub enum AgentDecisionsServing {
    NotServed,
    Served,
}

impl AgentDecisionsServing {
    #[must_use]
    pub const fn serves(&self, inference_capability: InferenceCapability) -> bool {
        match inference_capability {
            InferenceCapability::Decisions => match self {
                Self::NotServed => false,
                Self::Served => true,
            },
            InferenceCapability::Embeddings | InferenceCapability::TextGeneration => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::AgentDecisionsServing;
    use crate::inference_capability::InferenceCapability;

    #[test]
    fn only_an_agent_serving_decisions_serves_decisions() {
        assert!(!AgentDecisionsServing::NotServed.serves(InferenceCapability::Decisions));
        assert!(AgentDecisionsServing::Served.serves(InferenceCapability::Decisions));
    }

    #[test]
    fn every_serving_agent_serves_text_generation_and_embeddings() {
        for agent_decisions_serving in [
            AgentDecisionsServing::NotServed,
            AgentDecisionsServing::Served,
        ] {
            assert!(agent_decisions_serving.serves(InferenceCapability::Embeddings));
            assert!(agent_decisions_serving.serves(InferenceCapability::TextGeneration));
        }
    }
}
