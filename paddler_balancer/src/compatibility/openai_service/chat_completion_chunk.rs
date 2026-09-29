use serde::Serialize;
use serde::Serializer;
use serde_json::Value;

use crate::compatibility::openai_service::chat_completion_chunk_choice::ChatCompletionChunkChoice;
use crate::compatibility::openai_service::chat_completion_chunk_payload::ChatCompletionChunkPayload;
use crate::compatibility::openai_service::openai_usage_json::openai_usage_json;

#[derive(Serialize)]
struct ChunkEnvelope<'chunk, TBody> {
    id: &'chunk str,
    object: &'static str,
    created: u64,
    model: &'chunk str,
    system_fingerprint: &'chunk str,
    #[serde(flatten)]
    body: TBody,
}

#[derive(Serialize)]
struct ChoiceBody<'body, 'choice> {
    choices: [&'body ChatCompletionChunkChoice<'choice>; 1],
}

#[derive(Serialize)]
struct UsageBody {
    choices: [u8; 0],
    usage: Value,
}

pub struct ChatCompletionChunk<'chunk> {
    pub created: u64,
    pub id: &'chunk str,
    pub model: &'chunk str,
    pub payload: ChatCompletionChunkPayload<'chunk>,
    pub system_fingerprint: &'chunk str,
}

impl<'chunk> ChatCompletionChunk<'chunk> {
    const fn envelope<TBody>(&self, body: TBody) -> ChunkEnvelope<'chunk, TBody> {
        ChunkEnvelope {
            id: self.id,
            object: "chat.completion.chunk",
            created: self.created,
            model: self.model,
            system_fingerprint: self.system_fingerprint,
            body,
        }
    }
}

impl Serialize for ChatCompletionChunk<'_> {
    fn serialize<TSerializer>(
        &self,
        serializer: TSerializer,
    ) -> Result<TSerializer::Ok, TSerializer::Error>
    where
        TSerializer: Serializer,
    {
        match &self.payload {
            ChatCompletionChunkPayload::Choice(choice) => self
                .envelope(ChoiceBody { choices: [choice] })
                .serialize(serializer),
            ChatCompletionChunkPayload::Usage(usage) => self
                .envelope(UsageBody {
                    choices: [],
                    usage: openai_usage_json(usage),
                })
                .serialize(serializer),
        }
    }
}
