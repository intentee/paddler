#![cfg(feature = "tests_that_use_llms")]

use tokio_util::sync::CancellationToken;

use paddler_agent_pipeline_harness::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_agent_pipeline_harness::llama_runtime::LlamaRuntime;
use paddler_agent_text_generation::text_generation_error::TextGenerationError;
use paddler_agent_text_generation_tests::default_model_capabilities_settings::default_model_capabilities_settings;
use paddler_agent_text_generation_tests::text_generation_pipeline_parts::TextGenerationPipelineParts;
use paddler_inference_parameters::desired_sampling_parameters::DesiredSamplingParameters;
use paddler_inference_parameters::invalid_inference_parameters::InvalidInferenceParameters;
use paddler_inference_parameters::model_default_or::ModelDefaultOr;
use paddler_model_card_fixture::smolvlm2_256m::smolvlm2_256m;

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
    let model_capabilities = llama_runtime
        .establish_model_capabilities(default_model_capabilities_settings())
        .expect("the capabilities of the model must be established");
    let mut text_generation_pipeline = TextGenerationPipelineParts {
        llama_runtime,
        model_capabilities,
    }
    .into_text_generation_pipeline();

    text_generation_pipeline
        .text_generation_settings
        .sampling_parameters = DesiredSamplingParameters {
        penalty_last_n: ModelDefaultOr::Explicit(i32::MAX),
        penalty_repeat: ModelDefaultOr::Explicit(1.1),
        ..DesiredSamplingParameters::default()
    };

    assert!(matches!(
        text_generation_pipeline
            .spawn(&CancellationToken::new())
            .await
            .err(),
        Some(TextGenerationError::SamplingParametersUnfitForTheSequenceContext(
            InvalidInferenceParameters::PenaltyWindowExceedsSequenceContext {
                penalty_last_n: i32::MAX,
                sequence_context_size: refused_sequence_context_size,
            }
        )) if refused_sequence_context_size == sequence_context_size
    ));
}
