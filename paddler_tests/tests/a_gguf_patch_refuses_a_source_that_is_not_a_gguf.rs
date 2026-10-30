use std::fs::write;

use llama_cpp_bindings::gguf_context_error::GgufContextError;
use tempfile::TempDir;

use paddler_tests::point_qwen3_special_token_at_unused_padding::point_qwen3_special_token_at_unused_padding;
use paddler_tests::write_patched_gguf::write_patched_gguf;

#[test]
fn a_gguf_patch_refuses_a_source_that_is_not_a_gguf() {
    let model_directory = TempDir::new().expect("a temporary model directory must be creatable");
    let source_path = model_directory.path().join("text-that-is-not-a-gguf.gguf");

    write(&source_path, "text that is not a GGUF").expect("the source must be writable");

    assert_eq!(
        write_patched_gguf(
            &source_path,
            &model_directory.path().join("patched.gguf"),
            point_qwen3_special_token_at_unused_padding(c"tokenizer.ggml.eos_token_id"),
        )
        .err()
        .as_ref()
        .map(ToString::to_string),
        Some(GgufContextError::InitFailed(source_path).to_string())
    );
}
