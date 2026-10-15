use std::ffi::CStr;

use llama_cpp_bindings::gguf_context::GgufContext;

use crate::qwen3_0_6b_unused_padding_token::QWEN3_0_6B_UNUSED_PADDING_TOKEN;

pub fn point_qwen3_special_token_at_unused_padding(
    special_token_key: &CStr,
) -> impl FnOnce(&mut GgufContext) {
    move |gguf_context| gguf_context.set_val_u32(special_token_key, QWEN3_0_6B_UNUSED_PADDING_TOKEN)
}
