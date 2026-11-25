use std::sync::Arc;

use paddler_agent_model_capabilities::established_model_capabilities::EstablishedModelCapabilities;
use paddler_agent_runtime::loaded_llama_model::LoadedLlamaModel;

use crate::model_serving_settings::ModelServingSettings;
use crate::served_decisions::ServedDecisions;
use crate::served_embeddings::ServedEmbeddings;

pub enum LoadedModelServingPlan {
    RestartingTextGeneration {
        decisions: ServedDecisions,
        embeddings: ServedEmbeddings,
        loaded_llama_model: LoadedLlamaModel,
        model_capabilities: Arc<EstablishedModelCapabilities>,
        model_serving_settings: ModelServingSettings,
    },
    StartingEveryCapability {
        loaded_llama_model: LoadedLlamaModel,
        model_serving_settings: ModelServingSettings,
    },
}
