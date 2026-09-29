use std::slice::from_ref;

use serde::Serialize;
use serde::Serializer;
use serde::ser::SerializeMap as _;

use crate::compatibility::openai_service::chat_completion_chunk_payload::ChatCompletionChunkPayload;
use crate::compatibility::openai_service::openai_usage_json::openai_usage_json;

const NO_CHOICES: [u8; 0] = [];

pub struct ChatCompletionChunk<'chunk> {
    pub created: u64,
    pub id: &'chunk str,
    pub model: &'chunk str,
    pub payload: ChatCompletionChunkPayload<'chunk>,
    pub system_fingerprint: &'chunk str,
}

impl Serialize for ChatCompletionChunk<'_> {
    fn serialize<TSerializer>(
        &self,
        serializer: TSerializer,
    ) -> Result<TSerializer::Ok, TSerializer::Error>
    where
        TSerializer: Serializer,
    {
        let mut chunk = serializer.serialize_map(None)?;

        chunk.serialize_entry("id", self.id)?;
        chunk.serialize_entry("object", "chat.completion.chunk")?;
        chunk.serialize_entry("created", &self.created)?;
        chunk.serialize_entry("model", self.model)?;
        chunk.serialize_entry("system_fingerprint", self.system_fingerprint)?;

        match &self.payload {
            ChatCompletionChunkPayload::Choice(choice) => {
                chunk.serialize_entry("choices", from_ref(choice))?;
            }
            ChatCompletionChunkPayload::Usage(usage) => {
                chunk.serialize_entry("choices", &NO_CHOICES)?;
                chunk.serialize_entry("usage", &openai_usage_json(usage))?;
            }
        }

        chunk.end()
    }
}
