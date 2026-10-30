use serde::Serialize;
use serde::Serializer;

use crate::chat_completion_chunk_choice::ChatCompletionChunkChoice;
use crate::chat_completion_chunk_payload::ChatCompletionChunkPayload;
use crate::chat_completion_usage::ChatCompletionUsage;

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
struct ChoiceBody<'body> {
    choices: [&'body ChatCompletionChunkChoice; 1],
}

#[derive(Serialize)]
struct UsageBody {
    choices: [u8; 0],
    usage: ChatCompletionUsage,
}

pub struct ChatCompletionChunk {
    pub created: u64,
    pub id: String,
    pub model: String,
    pub payload: ChatCompletionChunkPayload,
    pub system_fingerprint: String,
}

impl ChatCompletionChunk {
    fn envelope<TBody>(&self, body: TBody) -> ChunkEnvelope<'_, TBody> {
        ChunkEnvelope {
            id: &self.id,
            object: "chat.completion.chunk",
            created: self.created,
            model: &self.model,
            system_fingerprint: &self.system_fingerprint,
            body,
        }
    }
}

impl Serialize for ChatCompletionChunk {
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
                    usage: ChatCompletionUsage(*usage),
                })
                .serialize(serializer),
        }
    }
}
