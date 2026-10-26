#![cfg(feature = "tests_that_use_llms")]

use tokio_util::sync::CancellationToken;

use paddler_agent_text_generation::multimodal_projection::MultimodalProjection;
use paddler_agent_text_generation::text_generation_error::TextGenerationError;
use paddler_inference_parameters::invalid_inference_parameters::InvalidInferenceParameters;
use paddler_inference_parameters::sampling_parameters::SamplingParameters;
use paddler_messaging::chat_template_source::ChatTemplateSource;
use paddler_test_cluster_harness::model_card::smolvlm2_256m::smolvlm2_256m;
use paddler_tests::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_tests::llama_runtime::LlamaRuntime;
use paddler_tests::text_generation_pipeline_parts::TextGenerationPipelineParts;

#[tokio::test(flavor = "multi_thread")]
async fn a_text_generation_pipeline_refuses_a_penalty_window_longer_than_its_sequence_context() {
    let llama_runtime = LlamaRuntime::load(
        &smolvlm2_256m()
            .cached_path()
            .expect("the model must be in the Hugging Face cache"),
        agent_inference_runtime_context(1),
    )
    .expect("the model must load");
    let sequence_context_size = llama_runtime
        .create_text_generation_llama_context(1)
        .expect("the context must be created")
        .n_ctx_seq();
    let mut text_generation_pipeline = TextGenerationPipelineParts {
        chat_template_source: ChatTemplateSource::EmbeddedInModel,
        llama_runtime,
        multimodal_projection: MultimodalProjection::NotConfigured,
    }
    .into_text_generation_pipeline();

    text_generation_pipeline
        .text_generation_settings
        .sampling_parameters = SamplingParameters {
        penalty_last_n: i32::MAX,
        penalty_repeat: 1.1,
        ..SamplingParameters::default()
    };

    assert_eq!(
        text_generation_pipeline
            .spawn(&CancellationToken::new())
            .await
            .err()
            .as_ref()
            .map(ToString::to_string),
        Some(
            TextGenerationError::SamplingParametersUnfitForTheSequenceContext(
                InvalidInferenceParameters::PenaltyWindowExceedsSequenceContext {
                    penalty_last_n: i32::MAX,
                    sequence_context_size,
                }
            )
            .to_string()
        )
    );
}
