#![cfg(feature = "tests_that_use_llms")]

use llama_cpp_bindings::chat_templates::ChatTemplates;
use llama_cpp_bindings::model::llama_chat_template::LlamaChatTemplate;

use paddler_agent_text_generation::generation_request_preparer::GenerationRequestPreparer;
use paddler_agent_text_generation::image_input::ImageInput;
use paddler_agent_text_generation::multimodal_prompt_support::MultimodalPromptSupport;
use paddler_tests::first_message_content_chat_template::first_message_content_chat_template;
use paddler_tests::llama_runtime::LlamaRuntime;

use crate::chat_templates_request_preparer::ChatTemplatesRequestPreparer;

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
        ChatTemplatesRequestPreparer {
            chat_templates: ChatTemplates::with_override(
                &self.llama_runtime.loaded_llama_model.model,
                &LlamaChatTemplate::new(&first_message_content_chat_template().content)
                    .expect("the first message content chat template holds no NUL"),
            )
            .expect("llama.cpp must prepare the first message content chat template"),
            image_input,
            llama_runtime: self.llama_runtime,
            sequence_context_size: self.sequence_context_size,
        }
        .into_generation_request_preparer()
    }
}
