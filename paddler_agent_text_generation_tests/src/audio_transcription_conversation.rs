use std::num::NonZeroU32;

use paddler_inference_parameters::model_default_or::ModelDefaultOr;
use paddler_messaging::conversation_history::ConversationHistory;
use paddler_messaging::conversation_message::ConversationMessage;
use paddler_messaging::conversation_message_content::ConversationMessageContent;
use paddler_messaging::conversation_message_content_part::ConversationMessageContentPart;
use paddler_messaging::input_audio::InputAudio;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;

#[must_use]
pub fn audio_transcription_conversation(
    input_audio: InputAudio,
) -> ContinueFromConversationHistoryParams<ValidatedParametersSchema> {
    ContinueFromConversationHistoryParams {
        add_generation_prompt: true,
        conversation_history: ConversationHistory::new(vec![ConversationMessage::Message {
            content: ConversationMessageContent::Parts(vec![
                ConversationMessageContentPart::InputAudio { input_audio },
                ConversationMessageContentPart::Text {
                    text: "Transcribe the audio.".to_owned(),
                },
            ]),
            role: "user".to_owned(),
        }]),
        enable_thinking: ModelDefaultOr::ModelDefault,
        grammar: None,
        max_tokens: NonZeroU32::MIN,
        parallel_tool_calls: ModelDefaultOr::ModelDefault,
        parse_tool_calls: false,
        tools: vec![],
    }
}
