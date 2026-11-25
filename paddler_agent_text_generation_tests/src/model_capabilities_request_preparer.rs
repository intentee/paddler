use anyhow::Result;

use paddler_agent_model_capabilities::model_capabilities_settings::ModelCapabilitiesSettings;
use paddler_agent_pipeline_harness::llama_runtime::LlamaRuntime;
use paddler_agent_text_generation::generation_request_preparer::GenerationRequestPreparer;
use paddler_agent_text_generation::model_bos_token::ModelBosToken;
use paddler_agent_text_generation::prompt_tokenizer::PromptTokenizer;
use paddler_messaging::multimodal_settings::MultimodalSettings;

pub struct ModelCapabilitiesRequestPreparer<'runtime> {
    pub llama_runtime: &'runtime LlamaRuntime,
    pub model_capabilities_settings: ModelCapabilitiesSettings,
    pub sequence_context_size: u32,
}

impl ModelCapabilitiesRequestPreparer<'_> {
    /// # Errors
    ///
    /// Returns an error when the chat template, the multimodal projection or the capabilities of the loaded model cannot be established.
    pub fn into_generation_request_preparer(self) -> Result<GenerationRequestPreparer> {
        let Self {
            llama_runtime,
            model_capabilities_settings,
            sequence_context_size,
        } = self;
        let loaded_llama_model = &llama_runtime.loaded_llama_model;
        let model = &loaded_llama_model.model;

        llama_runtime
            .establish_model_capabilities(model_capabilities_settings)
            .map(|model_capabilities| GenerationRequestPreparer {
                image_resize_to_fit: MultimodalSettings::default().image_resize_to_fit,
                loaded_llama_model: loaded_llama_model.clone(),
                prompt_tokenizer: PromptTokenizer {
                    model: model.clone(),
                    model_bos_token: ModelBosToken::of(model),
                    sequence_context_size,
                },
                sampler_chain_factory: llama_runtime.sampler_chain_factory(&model_capabilities),
                model_capabilities,
            })
    }
}
