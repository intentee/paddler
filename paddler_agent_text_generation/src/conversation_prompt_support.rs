use crate::chat_prompt_renderer::ChatPromptRenderer;
use crate::image_input::ImageInput;

pub struct ConversationPromptSupport {
    pub chat_prompt_renderer: ChatPromptRenderer,
    pub image_input: ImageInput,
}
