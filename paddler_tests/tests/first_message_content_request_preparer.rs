#![cfg(feature = "tests_that_use_llms")]

use paddler_agent_text_generation::chat_prompt_renderer::ChatPromptRenderer;
use paddler_agent_text_generation::chat_template_renderer::ChatTemplateRenderer;
use paddler_agent_text_generation::conversation_prompt_support::ConversationPromptSupport;
use paddler_agent_text_generation::conversation_support::ConversationSupport;
use paddler_agent_text_generation::generation_request_preparer::GenerationRequestPreparer;
use paddler_agent_text_generation::image_input::ImageInput;
use paddler_agent_text_generation::model_bos_token::ModelBosToken;
use paddler_agent_text_generation::multimodal_prompt_support::MultimodalPromptSupport;
use paddler_agent_text_generation::prompt_tokenizer::PromptTokenizer;
use paddler_messaging::multimodal_settings::MultimodalSettings;
use paddler_tests::first_message_content_chat_template::first_message_content_chat_template;
use paddler_tests::llama_runtime::LlamaRuntime;

pub struct FirstMessageContentRequestPreparer<'runtime> {
    pub llama_runtime: &'runtime LlamaRuntime,
    pub sequence_context_size: u32,
}

impl FirstMessageContentRequestPreparer<'_> {
    pub fn into_generation_request_preparer(self) -> GenerationRequestPreparer {
        self.into_preparer_with_image_input(ImageInput::Unsupported)
    }

    pub fn into_multimodal_generation_request_preparer(
        self,
        multimodal_prompt_support: MultimodalPromptSupport,
    ) -> GenerationRequestPreparer {
        self.into_preparer_with_image_input(ImageInput::Supported(multimodal_prompt_support))
    }

    fn into_preparer_with_image_input(self, image_input: ImageInput) -> GenerationRequestPreparer {
        let loaded_llama_model = &self.llama_runtime.loaded_llama_model;
        let model = &loaded_llama_model.model;

        GenerationRequestPreparer {
            conversation_support: ConversationSupport::Available(Box::new(
                ConversationPromptSupport {
                    chat_prompt_renderer: ChatPromptRenderer {
                        chat_template_renderer: ChatTemplateRenderer::new(
                            first_message_content_chat_template(),
                        )
                        .expect("the first message content chat template must compile"),
                        token_bos_str: String::new(),
                        token_eos_str: String::new(),
                        token_nl_str: String::new(),
                    },
                    image_input,
                },
            )),
            image_resize_to_fit: MultimodalSettings::default().image_resize_to_fit,
            loaded_llama_model: loaded_llama_model.clone(),
            prompt_tokenizer: PromptTokenizer {
                model: model.clone(),
                model_bos_token: ModelBosToken::of(model),
                sequence_context_size: self.sequence_context_size,
            },
            sampler_chain_factory: self.llama_runtime.sampler_chain_factory(),
            streaming_markers: model
                .streaming_markers()
                .expect("the model must report its streaming markers"),
        }
    }
}
