use anyhow::Context as _;
use anyhow::Result;
use llama_cpp_bindings::chat_templates::ChatTemplates;
use llama_cpp_bindings::model::llama_chat_template::LlamaChatTemplate;

use paddler_agent_text_generation::generation_request_preparer::GenerationRequestPreparer;
use paddler_agent_text_generation::image_input::ImageInput;
use paddler_agent_text_generation::multimodal_prompt_support::MultimodalPromptSupport;

use crate::chat_templates_request_preparer::ChatTemplatesRequestPreparer;
use crate::first_message_content_chat_template::first_message_content_chat_template;
use crate::llama_runtime::LlamaRuntime;

pub struct FirstMessageContentRequestPreparer<'runtime> {
    pub llama_runtime: &'runtime LlamaRuntime,
    pub sequence_context_size: u32,
}

impl FirstMessageContentRequestPreparer<'_> {
    /// # Errors
    ///
    /// Returns an error when the first message content chat template holds a NUL byte, and when llama.cpp cannot prepare it.
    pub fn into_generation_request_preparer(self) -> Result<GenerationRequestPreparer> {
        self.into_preparer_with_image_input(ImageInput::Unsupported)
    }

    /// # Errors
    ///
    /// Returns an error when the first message content chat template holds a NUL byte, and when llama.cpp cannot prepare it.
    pub fn into_multimodal_generation_request_preparer(
        self,
        multimodal_prompt_support: MultimodalPromptSupport,
    ) -> Result<GenerationRequestPreparer> {
        self.into_preparer_with_image_input(ImageInput::Supported(multimodal_prompt_support))
    }

    fn into_preparer_with_image_input(
        self,
        image_input: ImageInput,
    ) -> Result<GenerationRequestPreparer> {
        let Self {
            llama_runtime,
            sequence_context_size,
        } = self;

        LlamaChatTemplate::new(&first_message_content_chat_template().content)
            .context("the first message content chat template holds no NUL")
            .and_then(|first_message_content_chat_template| {
                ChatTemplates::with_override(
                    &llama_runtime.loaded_llama_model.model,
                    &first_message_content_chat_template,
                )
                .context("llama.cpp must prepare the first message content chat template")
            })
            .map(|chat_templates| {
                ChatTemplatesRequestPreparer {
                    chat_templates,
                    image_input,
                    llama_runtime,
                    sequence_context_size,
                }
                .into_generation_request_preparer()
            })
    }
}
