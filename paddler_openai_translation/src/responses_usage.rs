use serde::Serialize;
use serde::Serializer;

use paddler_messaging::token_usage::TokenUsage;

#[derive(Serialize)]
struct InputTokensDetails {
    cached_tokens: u64,
}

#[derive(Serialize)]
struct OutputTokensDetails {
    reasoning_tokens: u64,
}

#[derive(Serialize)]
struct SerializedUsage {
    input_tokens: u64,
    input_tokens_details: InputTokensDetails,
    output_tokens: u64,
    output_tokens_details: OutputTokensDetails,
    total_tokens: u64,
}

#[derive(Clone, Copy, Debug)]
pub struct ResponsesUsage(pub TokenUsage);

impl Serialize for ResponsesUsage {
    fn serialize<TSerializer>(
        &self,
        serializer: TSerializer,
    ) -> Result<TSerializer::Ok, TSerializer::Error>
    where
        TSerializer: Serializer,
    {
        let Self(usage) = self;

        SerializedUsage {
            input_tokens: usage.prompt,
            input_tokens_details: InputTokensDetails {
                cached_tokens: usage.cached_prompt,
            },
            output_tokens: usage.completion_tokens(),
            output_tokens_details: OutputTokensDetails {
                reasoning_tokens: usage.reasoning,
            },
            total_tokens: usage.total_tokens(),
        }
        .serialize(serializer)
    }
}
