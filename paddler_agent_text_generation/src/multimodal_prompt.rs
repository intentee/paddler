use llama_cpp_chat::chat_prompt_part::ChatPromptPart;
use llama_cpp_mtmd::mtmd_bitmap::MtmdBitmap;

use crate::media_marker::MediaMarker;

#[derive(Debug)]
pub struct MultimodalPrompt {
    pub bitmaps: Vec<MtmdBitmap>,
    pub media_marker: MediaMarker,
    pub prompt_parts: Vec<ChatPromptPart>,
}
