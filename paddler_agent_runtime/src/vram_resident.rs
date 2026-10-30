use paddler_messaging::inference_capability::InferenceCapability;

use crate::agent_runtime_error::AgentRuntimeError;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VramResident {
    LlamaContext(InferenceCapability),
    ModelWeights,
    MultimodalProjection,
}

impl VramResident {
    #[must_use]
    pub const fn already_resident(self) -> AgentRuntimeError {
        match self {
            Self::LlamaContext(inference_capability) => {
                AgentRuntimeError::LlamaContextAlreadyResident {
                    inference_capability,
                }
            }
            Self::ModelWeights => AgentRuntimeError::ModelWeightsAlreadyResident,
            Self::MultimodalProjection => AgentRuntimeError::MultimodalProjectionAlreadyResident,
        }
    }
}
