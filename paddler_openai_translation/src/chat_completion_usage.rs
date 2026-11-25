use serde::Serialize;
use serde::Serializer;

use paddler_messaging::token_usage::TokenUsage;

#[derive(Serialize)]
struct PromptTokensDetails {
    cached_tokens: u64,
    audio_tokens: u64,
}

#[derive(Serialize)]
struct CompletionTokensDetails {
    reasoning_tokens: u64,
}

#[derive(Serialize)]
struct SerializedUsage {
    prompt_tokens: u64,
    completion_tokens: u64,
    total_tokens: u64,
    prompt_tokens_details: PromptTokensDetails,
    completion_tokens_details: CompletionTokensDetails,
}

#[derive(Clone, Copy, Debug)]
pub struct ChatCompletionUsage(pub TokenUsage);

impl Serialize for ChatCompletionUsage {
    fn serialize<TSerializer>(
        &self,
        serializer: TSerializer,
    ) -> Result<TSerializer::Ok, TSerializer::Error>
    where
        TSerializer: Serializer,
    {
        let Self(usage) = self;

        SerializedUsage {
            prompt_tokens: usage.prompt,
            completion_tokens: usage.completion_tokens(),
            total_tokens: usage.total_tokens(),
            prompt_tokens_details: PromptTokensDetails {
                cached_tokens: usage.cached_prompt,
                audio_tokens: usage.input_audio,
            },
            completion_tokens_details: CompletionTokensDetails {
                reasoning_tokens: usage.reasoning,
            },
        }
        .serialize(serializer)
    }
}
