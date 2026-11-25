use llama_cpp_decision::decision_model::DecisionModel;

use paddler_agent_model_capabilities::established_model_capabilities::EstablishedModelCapabilities;
use paddler_agent_runtime::inference_runtime_context::InferenceRuntimeContext;
use paddler_agent_runtime::llama_context_settings::LlamaContextSettings;
use paddler_agent_runtime::loaded_llama_model::LoadedLlamaModel;
use paddler_agent_runtime::resident_llama_context::ResidentLlamaContext;
use paddler_messaging::inference_capability::InferenceCapability;

use crate::decision_error::DecisionError;

pub struct DecisionLlamaContextSettings<'settings> {
    pub decision_model: &'settings DecisionModel,
    pub llama_context_settings: LlamaContextSettings<'settings>,
    pub model_capabilities: &'settings EstablishedModelCapabilities,
}

impl DecisionLlamaContextSettings<'_> {
    /// # Errors
    ///
    /// Returns [`DecisionError::AgentRuntime`] when a decisions context already occupies the VRAM or llama.cpp cannot create the context.
    pub fn create_llama_context<'model>(
        self,
        loaded_llama_model: &'model LoadedLlamaModel,
        inference_runtime_context: &InferenceRuntimeContext,
    ) -> Result<ResidentLlamaContext<'model>, DecisionError> {
        Ok(loaded_llama_model.create_llama_context(
            inference_runtime_context,
            InferenceCapability::Decisions,
            self.model_capabilities.detected.context_params(
                self.decision_model.output().context_params(
                    self.llama_context_settings
                        .into_llama_context_params()
                        .with_kv_unified(true),
                ),
            ),
        )?)
    }
}
