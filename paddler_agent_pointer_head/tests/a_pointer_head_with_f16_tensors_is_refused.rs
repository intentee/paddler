use std::error::Error;

use llama_cpp_bindings::context::kv_cache_type::KvCacheType;
use llama_cpp_bindings::gguf_context_error::GgufContextError;

use paddler_agent_pointer_head::pointer_head::PointerHead;
use paddler_agent_pointer_head::pointer_head_error::PointerHeadError;

use crate::fixture_path::fixture_path;

fn key_bias_is_not_f32() -> GgufContextError {
    GgufContextError::TensorIsNotF32 {
        name: "pointer_head.key.bias".to_owned(),
        ggml_type: KvCacheType::F16.into(),
    }
}

#[test]
fn a_pointer_head_with_f16_tensors_is_refused() {
    let load_error = PointerHead::load(&fixture_path("pointer_head_with_f16_tensors.gguf")).err();

    assert_eq!(
        load_error.as_ref().map(ToString::to_string),
        Some(
            PointerHeadError::TensorUnreadable {
                tensor: "pointer_head.key.bias".to_owned(),
                source: key_bias_is_not_f32(),
            }
            .to_string()
        )
    );
    assert_eq!(
        load_error
            .as_ref()
            .and_then(Error::source)
            .and_then(|source| source.downcast_ref::<GgufContextError>())
            .map(ToString::to_string),
        Some(key_bias_is_not_f32().to_string())
    );
}
