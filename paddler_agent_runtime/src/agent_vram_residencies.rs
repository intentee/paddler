use paddler_messaging::inference_capability::InferenceCapability;

use crate::vram_residency::VramResidency;
use crate::vram_resident::VramResident;

#[derive(Clone)]
pub struct AgentVramResidencies {
    pub decisions_context: VramResidency,
    pub embeddings_context: VramResidency,
    pub model_weights: VramResidency,
    pub multimodal_projection: VramResidency,
    pub text_generation_context: VramResidency,
}

impl AgentVramResidencies {
    #[must_use]
    pub const fn llama_context(&self, inference_capability: InferenceCapability) -> &VramResidency {
        match inference_capability {
            InferenceCapability::Decisions => &self.decisions_context,
            InferenceCapability::Embeddings => &self.embeddings_context,
            InferenceCapability::TextGeneration => &self.text_generation_context,
        }
    }
}

impl Default for AgentVramResidencies {
    fn default() -> Self {
        Self {
            decisions_context: VramResidency::vacant(VramResident::LlamaContext(
                InferenceCapability::Decisions,
            )),
            embeddings_context: VramResidency::vacant(VramResident::LlamaContext(
                InferenceCapability::Embeddings,
            )),
            model_weights: VramResidency::vacant(VramResident::ModelWeights),
            multimodal_projection: VramResidency::vacant(VramResident::MultimodalProjection),
            text_generation_context: VramResidency::vacant(VramResident::LlamaContext(
                InferenceCapability::TextGeneration,
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use paddler_messaging::inference_capability::InferenceCapability;

    use super::AgentVramResidencies;
    use crate::agent_runtime_error::AgentRuntimeError;

    #[test]
    fn every_capability_occupies_a_context_residency_of_its_own() {
        let inference_capabilities = [
            InferenceCapability::Decisions,
            InferenceCapability::Embeddings,
            InferenceCapability::TextGeneration,
        ];
        let agent_vram_residencies = AgentVramResidencies::default();
        let _context_occupancies = inference_capabilities.map(|inference_capability| {
            agent_vram_residencies
                .llama_context(inference_capability)
                .try_occupy()
                .unwrap()
        });

        assert_eq!(
            inference_capabilities.map(|inference_capability| agent_vram_residencies
                .llama_context(inference_capability)
                .try_occupy()
                .err()
                .map(|error| error.to_string())),
            inference_capabilities.map(|inference_capability| Some(
                AgentRuntimeError::LlamaContextAlreadyResident {
                    inference_capability
                }
                .to_string()
            ))
        );
    }
}
