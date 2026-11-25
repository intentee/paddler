#![cfg(feature = "tests_that_use_llms")]

use llama_cpp_model::model_error::ModelError;
use tempfile::TempDir;

use paddler_agent_pipeline_harness::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_agent_pipeline_harness::llama_runtime::LlamaRuntime;
use paddler_agent_pipeline_harness::write_patched_gguf::write_patched_gguf;
use paddler_agent_runtime::agent_runtime_error::AgentRuntimeError;
use paddler_model_card_fixture::qwen3_0_6b::qwen3_0_6b;

#[test]
fn a_model_with_a_metadata_value_that_is_not_utf8_reports_unreadable_metadata() {
    let model_directory = TempDir::new().expect("a temporary model directory must be creatable");
    let model_path = model_directory
        .path()
        .join("qwen3-with-a-value-that-is-not-utf8.gguf");

    write_patched_gguf(
        &qwen3_0_6b()
            .cached_path()
            .expect("Qwen3 must be in the Hugging Face cache"),
        &model_path,
        |gguf_context| {
            gguf_context.set_val_str(c"general.name", c"\xff");

            Ok(())
        },
    )
    .expect("the patched model must be writable");

    assert!(matches!(
        LlamaRuntime::load(&model_path, agent_inference_runtime_context(1)),
        Err(AgentRuntimeError::ModelMetadataUnreadable(
            ModelError::MetaValueNotUtf8(_)
        ))
    ));
}
