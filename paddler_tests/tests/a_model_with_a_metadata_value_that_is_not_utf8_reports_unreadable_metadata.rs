#![cfg(feature = "tests_that_use_llms")]

use hf_hub::Cache;
use llama_cpp_bindings::error::MetaValError;
use tempfile::TempDir;

use paddler_agent_runtime::agent_runtime_error::AgentRuntimeError;
use paddler_agent_runtime::describe_error_with_causes::describe_error_with_causes;
use paddler_test_cluster_harness::model_card::qwen3_0_6b::qwen3_0_6b;
use paddler_tests::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_tests::llama_runtime::LlamaRuntime;
use paddler_tests::write_patched_gguf::write_patched_gguf;

#[test]
fn a_model_with_a_metadata_value_that_is_not_utf8_reports_unreadable_metadata() {
    let model_directory = TempDir::new().expect("a temporary model directory must be creatable");
    let model_path = model_directory
        .path()
        .join("qwen3-with-a-value-that-is-not-utf8.gguf");

    write_patched_gguf(
        &qwen3_0_6b()
            .reference
            .cached_path(&Cache::from_env())
            .expect("Qwen3 must be in the Hugging Face cache"),
        &model_path,
        |gguf_context| {
            gguf_context.set_val_str(c"general.name", c"\xff");
        },
    )
    .expect("the patched model must be writable");

    assert_eq!(
        LlamaRuntime::load(&model_path, agent_inference_runtime_context(1))
            .err()
            .as_ref()
            .map(|load_error| describe_error_with_causes(load_error)),
        Some(describe_error_with_causes(
            &AgentRuntimeError::ModelMetadataUnreadable(MetaValError::FromUtf8Error(
                String::from_utf8(vec![0xff]).expect_err("the bytes must not be UTF-8")
            ))
        ))
    );
}
