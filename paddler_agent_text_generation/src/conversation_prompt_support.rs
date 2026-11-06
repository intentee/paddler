use llama_cpp_bindings::chat_templates::ChatTemplates;

use crate::image_input::ImageInput;

pub struct ConversationPromptSupport {
    pub chat_templates: ChatTemplates,
    pub image_input: ImageInput,
}
