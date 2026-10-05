use crate::multimodal_prompt_support::MultimodalPromptSupport;

pub enum PromptModality<'support> {
    Multimodal(&'support MultimodalPromptSupport),
    TextOnly,
}
