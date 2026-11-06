#![cfg(feature = "tests_that_use_llms")]

use llama_cpp_bindings::chat_templates::ChatTemplates;

use paddler_agent_text_generation::conversation_prompt_support::ConversationPromptSupport;
use paddler_agent_text_generation::conversation_support::ConversationSupport;
use paddler_agent_text_generation::generation_request_preparer::GenerationRequestPreparer;
use paddler_agent_text_generation::image_input::ImageInput;
use paddler_agent_text_generation::model_bos_token::ModelBosToken;
use paddler_agent_text_generation::prompt_tokenizer::PromptTokenizer;
use paddler_messaging::multimodal_settings::MultimodalSettings;
use paddler_tests::llama_runtime::LlamaRuntime;

pub struct ChatTemplatesRequestPreparer<'runtime> {
    pub chat_templates: ChatTemplates,
    pub image_input: ImageInput,
    pub llama_runtime: &'runtime LlamaRuntime,
    pub sequence_context_size: u32,
}

impl ChatTemplatesRequestPreparer<'_> {
    pub fn into_generation_request_preparer(self) -> GenerationRequestPreparer {
        let Self {
            chat_templates,
            image_input,
            llama_runtime,
            sequence_context_size,
        } = self;
        let loaded_llama_model = &llama_runtime.loaded_llama_model;
        let model = &loaded_llama_model.model;

        GenerationRequestPreparer {
            conversation_support: ConversationSupport::Available(Box::new(
                ConversationPromptSupport {
                    chat_templates,
                    image_input,
                },
            )),
            image_resize_to_fit: MultimodalSettings::default().image_resize_to_fit,
            loaded_llama_model: loaded_llama_model.clone(),
            prompt_tokenizer: PromptTokenizer {
                model: model.clone(),
                model_bos_token: ModelBosToken::of(model),
                sequence_context_size,
            },
            sampler_chain_factory: llama_runtime.sampler_chain_factory(),
        }
    }
}
