use paddler_agent_runtime::loaded_llama_model::LoadedLlamaModel;

use crate::capability_startup_interruption::CapabilityStartupInterruption;

pub struct ServingInterruption {
    pub interruption: CapabilityStartupInterruption,
    pub loaded_llama_model: LoadedLlamaModel,
}
