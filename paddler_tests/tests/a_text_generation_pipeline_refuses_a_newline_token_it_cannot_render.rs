#![cfg(feature = "tests_that_use_llms")]

use hf_hub::Cache;
use llama_cpp_bindings::error::TokenToStringError;
use llama_cpp_bindings_sys::LLAMA_TOKEN_TYPE_UNUSED;
use tempfile::TempDir;

use paddler_agent_text_generation::llama_cpp_media_marker::LlamaCppMediaMarker;
use paddler_agent_text_generation::multimodal_projection::MultimodalProjection;
use paddler_agent_text_generation::text_generation_error::TextGenerationError;
use paddler_messaging::chat_template_source::ChatTemplateSource;
use paddler_test_cluster_harness::model_card::qwen3_0_6b::qwen3_0_6b;
use paddler_tests::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_tests::qwen3_0_6b_newline_token::QWEN3_0_6B_NEWLINE_TOKEN;
use paddler_tests::text_generation_pipeline_parts::TextGenerationPipelineParts;
use paddler_tests::write_patched_gguf::write_patched_gguf;

#[tokio::test(flavor = "multi_thread")]
async fn a_text_generation_pipeline_refuses_a_newline_token_it_cannot_render() {
    let model_directory = TempDir::new().expect("a temporary model directory must be creatable");
    let model_path = model_directory
        .path()
        .join("qwen3-with-an-unrenderable-newline-token.gguf");

    write_patched_gguf(
        &qwen3_0_6b()
            .reference
            .cached_path(&Cache::from_env())
            .expect("Qwen3 must be in the Hugging Face cache"),
        &model_path,
        |gguf_context| {
            let token_types_key_id = gguf_context
                .find_key("tokenizer.ggml.token_type")
                .expect("Qwen3 must declare its token types");
            let mut token_types = gguf_context
                .val_arr_i32(token_types_key_id)
                .expect("Qwen3 token types must be an i32 array")
                .to_vec();

            token_types[QWEN3_0_6B_NEWLINE_TOKEN] = i32::try_from(LLAMA_TOKEN_TYPE_UNUSED)
                .expect("the unused token type must fit the GGUF token type array");
            gguf_context.set_arr_i32(c"tokenizer.ggml.token_type", &token_types);
        },
    )
    .expect("the model with an unrenderable newline token must be writable");

    assert_eq!(
        TextGenerationPipelineParts {
            chat_template_source: ChatTemplateSource::EmbeddedInModel,
            inference_runtime_context: agent_inference_runtime_context(1),
            media_marker: LlamaCppMediaMarker,
            model_path,
            multimodal_projection: MultimodalProjection::NotConfigured,
        }
        .spawn_refusal()
        .await,
        Some(
            TextGenerationError::SpecialTokenUnrenderable(TokenToStringError::UnknownTokenType)
                .to_string()
        )
    );
}
