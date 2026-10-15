#![cfg(feature = "tests_that_use_llms")]

use hf_hub::Cache;

use paddler_agent_text_generation::multimodal_projection::MultimodalProjection;
use paddler_agent_text_generation::provides_media_marker::ProvidesMediaMarker;
use paddler_agent_text_generation::text_generation_error::TextGenerationError;
use paddler_messaging::chat_template_source::ChatTemplateSource;
use paddler_test_cluster_harness::model_card::qwen3_0_6b::qwen3_0_6b;
use paddler_tests::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_tests::media_marker_that_is_not_utf8::MediaMarkerThatIsNotUtf8;
use paddler_tests::text_generation_pipeline_parts::TextGenerationPipelineParts;

#[tokio::test(flavor = "multi_thread")]
async fn a_text_generation_pipeline_refuses_a_media_marker_that_is_not_utf8() {
    assert_eq!(
        TextGenerationPipelineParts {
            chat_template_source: ChatTemplateSource::EmbeddedInModel,
            inference_runtime_context: agent_inference_runtime_context(1),
            media_marker: MediaMarkerThatIsNotUtf8,
            model_path: qwen3_0_6b()
                .reference
                .cached_path(&Cache::from_env())
                .expect("Qwen3 must be in the Hugging Face cache"),
            multimodal_projection: MultimodalProjection::NotConfigured,
        }
        .spawn_refusal()
        .await,
        Some(
            TextGenerationError::MediaMarkerUnavailable(
                MediaMarkerThatIsNotUtf8
                    .media_marker()
                    .expect_err("the fake media marker must not be UTF-8")
            )
            .to_string()
        )
    );
}
