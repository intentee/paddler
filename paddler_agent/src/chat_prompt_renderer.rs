use minijinja::context;
use paddler_messaging::media_marker::MediaMarker;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;

use crate::chat_template_renderer::ChatTemplateRenderer;
use crate::generation_request_rejection::GenerationRequestRejection;

pub struct ChatPromptRenderer {
    pub chat_template_renderer: ChatTemplateRenderer,
    pub media_marker: MediaMarker,
    pub token_bos_str: String,
    pub token_eos_str: String,
    pub token_nl_str: String,
}

impl ChatPromptRenderer {
    pub fn render(
        &self,
        ContinueFromConversationHistoryParams {
            add_generation_prompt,
            conversation_history,
            enable_thinking,
            tools,
            ..
        }: &ContinueFromConversationHistoryParams<ValidatedParametersSchema>,
    ) -> Result<String, GenerationRequestRejection> {
        self.chat_template_renderer
            .render(context! {
                add_generation_prompt,
                bos_token => self.token_bos_str,
                enable_thinking,
                eos_token => self.token_eos_str,
                messages => conversation_history.replace_images_with_marker(&self.media_marker).messages,
                nl_token => self.token_nl_str,
                tools,
            })
            .map_err(GenerationRequestRejection::ChatTemplateRenderingFailed)
    }
}

#[cfg(test)]
mod tests {
    use std::mem::discriminant;

    use anyhow::anyhow;
    use paddler_messaging::chat_template::ChatTemplate;
    use paddler_messaging::conversation_history::ConversationHistory;
    use paddler_messaging::conversation_message::ConversationMessage;
    use paddler_messaging::conversation_message_content::ConversationMessageContent;
    use paddler_messaging::conversation_message_content_part::ConversationMessageContentPart;
    use paddler_messaging::image_url::ImageUrl;
    use paddler_messaging::media_marker::MediaMarker;
    use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
    use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;

    use super::ChatPromptRenderer;
    use crate::chat_template_renderer::ChatTemplateRenderer;
    use crate::generation_request_rejection::GenerationRequestRejection;

    fn renderer_for(template_content: &str) -> ChatPromptRenderer {
        ChatPromptRenderer {
            chat_template_renderer: ChatTemplateRenderer::new(ChatTemplate {
                content: template_content.to_owned(),
            })
            .unwrap(),
            media_marker: MediaMarker::new("<media>".to_owned()),
            token_bos_str: "<bos>".to_owned(),
            token_eos_str: "<eos>".to_owned(),
            token_nl_str: "<nl>".to_owned(),
        }
    }

    fn image_then_text_params() -> ContinueFromConversationHistoryParams<ValidatedParametersSchema>
    {
        ContinueFromConversationHistoryParams {
            add_generation_prompt: true,
            conversation_history: ConversationHistory::new(vec![ConversationMessage {
                content: ConversationMessageContent::Parts(vec![
                    ConversationMessageContentPart::ImageUrl {
                        image_url: ImageUrl {
                            url: "data:image/png;base64,AAAA".to_owned(),
                        },
                    },
                    ConversationMessageContentPart::Text {
                        text: "Describe".to_owned(),
                    },
                ]),
                role: "user".to_owned(),
            }]),
            enable_thinking: false,
            grammar: None,
            max_tokens: 1,
            parse_tool_calls: false,
            tools: vec![],
        }
    }

    #[test]
    fn renders_images_as_media_markers_between_special_tokens() {
        let renderer = renderer_for(
            "{{ bos_token }}{% for message in messages %}{% for part in message.content %}{{ part.text }}{{ nl_token }}{% endfor %}{% endfor %}{{ eos_token }}",
        );

        assert_eq!(
            renderer.render(&image_then_text_params()).unwrap(),
            "<bos><media><nl>Describe<nl><eos>"
        );
    }

    #[test]
    fn reports_template_failures_as_rendering_rejections() {
        let renderer = renderer_for("{{ raise_exception('unsupported conversation') }}");

        assert_eq!(
            renderer
                .render(&image_then_text_params())
                .err()
                .map(|rejection| discriminant(&rejection)),
            Some(discriminant(
                &GenerationRequestRejection::ChatTemplateRenderingFailed(anyhow!("expected"))
            ))
        );
    }
}
