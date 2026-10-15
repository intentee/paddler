#![cfg(feature = "tests_that_use_llms")]

use hf_hub::Cache;
use llama_cpp_bindings::error::MarkerDetectionError;
use tempfile::TempDir;

use paddler_agent_text_generation::llama_cpp_media_marker::LlamaCppMediaMarker;
use paddler_agent_text_generation::multimodal_projection::MultimodalProjection;
use paddler_agent_text_generation::text_generation_error::TextGenerationError;
use paddler_messaging::chat_template_source::ChatTemplateSource;
use paddler_test_cluster_harness::model_card::qwen3_0_6b::qwen3_0_6b;
use paddler_tests::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_tests::first_message_content_chat_template::first_message_content_chat_template;
use paddler_tests::text_generation_pipeline_parts::TextGenerationPipelineParts;
use paddler_tests::write_patched_gguf::write_patched_gguf;

#[tokio::test(flavor = "multi_thread")]
async fn a_text_generation_pipeline_refuses_a_model_whose_embedded_template_hides_its_streaming_markers()
 {
    let model_directory = TempDir::new().expect("a temporary model directory must be creatable");
    let model_path = model_directory
        .path()
        .join("qwen3-with-an-embedded-template-that-does-not-parse.gguf");

    write_patched_gguf(
        &qwen3_0_6b()
            .reference
            .cached_path(&Cache::from_env())
            .expect("Qwen3 must be in the Hugging Face cache"),
        &model_path,
        |gguf_context| {
            gguf_context.set_val_str(c"tokenizer.chat_template", c"{% if %}");
        },
    )
    .expect("the model with a template that does not parse must be writable");

    assert_eq!(
        TextGenerationPipelineParts {
            chat_template_source: ChatTemplateSource::Override(
                first_message_content_chat_template()
            ),
            inference_runtime_context: agent_inference_runtime_context(1),
            media_marker: LlamaCppMediaMarker,
            model_path,
            multimodal_projection: MultimodalProjection::NotConfigured,
        }
        .spawn_refusal()
        .await,
        Some(
            TextGenerationError::StreamingMarkersUnavailable(
                MarkerDetectionError::ReasoningMarkerDetectionFailed {
                    message: "Unexpected token: %} of type 9".to_owned(),
                }
            )
            .to_string()
        )
    );
}
