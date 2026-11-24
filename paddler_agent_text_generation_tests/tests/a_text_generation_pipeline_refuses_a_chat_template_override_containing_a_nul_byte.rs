#![cfg(feature = "tests_that_use_llms")]

use paddler_agent_pipeline_harness::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_agent_pipeline_harness::llama_runtime::LlamaRuntime;
use paddler_agent_text_generation::multimodal_projection::MultimodalProjection;
use paddler_agent_text_generation::text_generation_error::TextGenerationError;
use paddler_agent_text_generation_tests::text_generation_pipeline_parts::TextGenerationPipelineParts;
use paddler_messaging::chat_template::ChatTemplate;
use paddler_messaging::chat_template_source::ChatTemplateSource;
use paddler_model_card_fixture::qwen3_0_6b::qwen3_0_6b;

const TEMPLATE_WITH_A_NUL_BYTE: &str = "{{ messages[0].content }}\0";

#[tokio::test(flavor = "multi_thread")]
async fn a_text_generation_pipeline_refuses_a_chat_template_override_containing_a_nul_byte() {
    assert!(matches!(
        TextGenerationPipelineParts {
            chat_template_source: ChatTemplateSource::Override(ChatTemplate {
                content: TEMPLATE_WITH_A_NUL_BYTE.to_owned(),
            }),
            llama_runtime: LlamaRuntime::load(
                &qwen3_0_6b()
                    .cached_path()
                    .expect("the model must be in the Hugging Face cache"),
                agent_inference_runtime_context(1),
            )
            .expect("the model must load"),
            multimodal_projection: MultimodalProjection::NotConfigured,
        }
        .spawn_refusal()
        .await,
        Some(TextGenerationError::ChatTemplateContainsNul(_))
    ));
}
