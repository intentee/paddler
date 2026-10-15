#![cfg(feature = "tests_that_use_llms")]

use hf_hub::Cache;

use paddler_agent_text_generation::llama_cpp_media_marker::LlamaCppMediaMarker;
use paddler_agent_text_generation::multimodal_projection::MultimodalProjection;
use paddler_messaging::chat_template::ChatTemplate;
use paddler_messaging::chat_template_source::ChatTemplateSource;
use paddler_test_cluster_harness::model_card::smolvlm2_256m::smolvlm2_256m;
use paddler_test_cluster_harness::model_card::smolvlm2_256m_mmproj::smolvlm2_256m_mmproj;
use paddler_tests::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_tests::text_generation_pipeline_parts::TextGenerationPipelineParts;

#[tokio::test(flavor = "multi_thread")]
async fn a_multimodal_pipeline_refuses_a_template_that_raises_while_probing_its_bos_token() {
    assert_eq!(
        TextGenerationPipelineParts {
            chat_template_source: ChatTemplateSource::Override(ChatTemplate {
                content: "{{ raise_exception('the template refuses to render') }}".to_owned(),
            }),
            inference_runtime_context: agent_inference_runtime_context(1),
            media_marker: LlamaCppMediaMarker,
            model_path: smolvlm2_256m()
                .reference
                .cached_path(&Cache::from_env())
                .expect("the vision model must be in the Hugging Face cache"),
            multimodal_projection: MultimodalProjection::File(
                smolvlm2_256m_mmproj()
                    .reference
                    .cached_path(&Cache::from_env())
                    .expect("the multimodal projection must be in the Hugging Face cache"),
            ),
        }
        .spawn_refusal()
        .await,
        Some(
            "failed to render chat template: invalid operation: Model's chat template raised an exception: 'the template refuses to render' (in chat_template:1)"
                .to_owned()
        )
    );
}
