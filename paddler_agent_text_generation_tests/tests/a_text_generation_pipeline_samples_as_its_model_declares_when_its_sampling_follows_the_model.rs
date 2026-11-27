#![cfg(feature = "tests_that_use_llms")]

use tempfile::TempDir;
use tokio_util::sync::CancellationToken;

use paddler_agent_pipeline_harness::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_agent_pipeline_harness::llama_runtime::LlamaRuntime;
use paddler_agent_pipeline_harness::write_patched_gguf::write_patched_gguf;
use paddler_agent_runtime::scheduler_handle::SchedulerHandle;
use paddler_agent_runtime::shuts_down::ShutsDown as _;
use paddler_agent_runtime::startup_outcome::StartupOutcome;
use paddler_agent_text_generation_tests::default_model_capabilities_settings::default_model_capabilities_settings;
use paddler_agent_text_generation_tests::text_generation_pipeline_parts::TextGenerationPipelineParts;
use paddler_model_card_fixture::smolvlm2_256m::smolvlm2_256m;

const DECLARED_TEMPERATURE: f32 = 0.25;

#[tokio::test(flavor = "multi_thread")]
async fn a_text_generation_pipeline_samples_as_its_model_declares_when_its_sampling_follows_the_model()
 {
    let model_directory = TempDir::new().expect("a temporary model directory must be creatable");
    let model_declaring_sampling_path = model_directory
        .path()
        .join("smolvlm2-declaring-its-sampling.gguf");

    write_patched_gguf(
        &smolvlm2_256m()
            .cached_path()
            .expect("the model must be in the Hugging Face cache"),
        &model_declaring_sampling_path,
        |gguf_context| {
            gguf_context.set_val_str(c"general.sampling.temp", c"0.25");

            Ok(())
        },
    )
    .expect("the model declaring its sampling must be writable");

    let llama_runtime = LlamaRuntime::load(
        &model_declaring_sampling_path,
        agent_inference_runtime_context(1),
    )
    .expect("the model must load");
    let model_capabilities = llama_runtime
        .establish_model_capabilities(default_model_capabilities_settings())
        .expect("the capabilities of the model must be established");

    let text_generation_startup = TextGenerationPipelineParts {
        llama_runtime,
        model_capabilities,
    }
    .into_text_generation_pipeline()
    .spawn(&CancellationToken::new())
    .await
    .expect("the text generation pipeline must start");
    let started_temperature = match &text_generation_startup {
        StartupOutcome::Ready(SchedulerHandle {
            request_preparer, ..
        }) => Some(
            request_preparer
                .preparation
                .sampler_chain_factory
                .sampling_parameters
                .temperature
                .to_bits(),
        ),
        StartupOutcome::Cancelled => None,
    };

    if let StartupOutcome::Ready(scheduler_handle) = text_generation_startup {
        scheduler_handle.shut_down().await;
    }

    assert_eq!(started_temperature, Some(DECLARED_TEMPERATURE.to_bits()));
}
