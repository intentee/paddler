use std::num::NonZeroU32;

use paddler_messaging::conversation_history::ConversationHistory;
use paddler_messaging::conversation_message::ConversationMessage;
use paddler_messaging::conversation_message_content::ConversationMessageContent;
use paddler_messaging::conversation_message_content_part::ConversationMessageContentPart;
use paddler_messaging::image_url::ImageUrl;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;

pub struct SingleImageQuestion {
    pub image_data_uri: String,
    pub max_tokens: NonZeroU32,
    pub question: &'static str,
}

impl SingleImageQuestion {
    #[must_use]
    pub fn into_conversation_params(
        self,
    ) -> ContinueFromConversationHistoryParams<ValidatedParametersSchema> {
        let Self {
            image_data_uri,
            max_tokens,
            question,
        } = self;

        ContinueFromConversationHistoryParams {
            add_generation_prompt: true,
            conversation_history: ConversationHistory::new(vec![ConversationMessage {
                content: ConversationMessageContent::Parts(vec![
                    ConversationMessageContentPart::ImageUrl {
                        image_url: ImageUrl {
                            url: image_data_uri,
                        },
                    },
                    ConversationMessageContentPart::Text {
                        text: question.to_owned(),
                    },
                ]),
                role: "user".to_owned(),
            }]),
            enable_thinking: false,
            grammar: None,
            max_tokens,
            parse_tool_calls: false,
            tools: vec![],
        }
    }
}
