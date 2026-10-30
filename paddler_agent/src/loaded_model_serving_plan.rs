use paddler_agent_runtime::loaded_llama_model::LoadedLlamaModel;

use crate::capability_plan::CapabilityPlan;
use crate::inference_capabilities::InferenceCapabilities;
use crate::model_serving_settings::ModelServingSettings;
use crate::served_decisions::ServedDecisions;
use crate::served_embeddings::ServedEmbeddings;
use crate::served_text_generation::ServedTextGeneration;

pub struct LoadedModelServingPlan {
    pub capability_plans: InferenceCapabilities<
        CapabilityPlan<ServedDecisions>,
        CapabilityPlan<ServedEmbeddings>,
        CapabilityPlan<ServedTextGeneration>,
    >,
    pub loaded_llama_model: LoadedLlamaModel,
    pub model_serving_settings: ModelServingSettings,
}

impl LoadedModelServingPlan {
    #[must_use]
    pub const fn starting_every_capability(
        loaded_llama_model: LoadedLlamaModel,
        model_serving_settings: ModelServingSettings,
    ) -> Self {
        Self {
            capability_plans: InferenceCapabilities {
                decisions: CapabilityPlan::Start,
                embeddings: CapabilityPlan::Start,
                text_generation: CapabilityPlan::Start,
            },
            loaded_llama_model,
            model_serving_settings,
        }
    }
}
