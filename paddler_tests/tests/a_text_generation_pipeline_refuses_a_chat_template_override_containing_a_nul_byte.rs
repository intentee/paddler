#![cfg(feature = "tests_that_use_llms")]

use std::ffi::CString;

use paddler_agent_text_generation::multimodal_projection::MultimodalProjection;
use paddler_agent_text_generation::text_generation_error::TextGenerationError;
use paddler_messaging::chat_template::ChatTemplate;
use paddler_messaging::chat_template_source::ChatTemplateSource;
use paddler_test_cluster_harness::model_card::qwen3_0_6b::qwen3_0_6b;
use paddler_tests::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_tests::llama_runtime::LlamaRuntime;
use paddler_tests::text_generation_pipeline_parts::TextGenerationPipelineParts;

const TEMPLATE_WITH_A_NUL_BYTE: &str = "{{ messages[0].content }}\0";

#[tokio::test(flavor = "multi_thread")]
async fn a_text_generation_pipeline_refuses_a_chat_template_override_containing_a_nul_byte() {
    assert_eq!(
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
        Some(
            TextGenerationError::ChatTemplateContainsNul(
                CString::new(TEMPLATE_WITH_A_NUL_BYTE)
                    .expect_err("the template must hold a NUL byte")
            )
            .to_string()
        )
    );
}
