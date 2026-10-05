use llama_cpp_bindings_types::TokenUsage;
use serde::Serialize;
use serde::Serializer;

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
pub struct OpenAIUsage(pub TokenUsage);

impl Serialize for OpenAIUsage {
    fn serialize<TSerializer>(
        &self,
        serializer: TSerializer,
    ) -> Result<TSerializer::Ok, TSerializer::Error>
    where
        TSerializer: Serializer,
    {
        let Self(usage) = self;

        SerializedUsage {
            prompt_tokens: usage.prompt_tokens,
            completion_tokens: usage.completion_tokens(),
            total_tokens: usage.total_tokens(),
            prompt_tokens_details: PromptTokensDetails {
                cached_tokens: usage.cached_prompt_tokens,
                audio_tokens: usage.input_audio_tokens,
            },
            completion_tokens_details: CompletionTokensDetails {
                reasoning_tokens: usage.reasoning_tokens,
            },
        }
        .serialize(serializer)
    }
}
