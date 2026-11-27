#![cfg(feature = "tests_that_use_llms")]

use llama_cpp_params::fit_defaults::FitDefaults;

use paddler_agent_pipeline_harness::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_agent_runtime::llama_model_settings::LlamaModelSettings;
use paddler_agent_runtime::loaded_llama_model::LoadedLlamaModel;
use paddler_agent_runtime::model_placement::ModelPlacement;
use paddler_inference_parameters::desired_model_runtime_parameters::DesiredModelRuntimeParameters;
use paddler_messaging::inference_capability::InferenceCapability;
use paddler_model_card_fixture::smolvlm2_256m::smolvlm2_256m;

#[test]
fn a_model_whose_context_follows_the_model_gets_the_context_llama_cpp_fits_to_device_memory() {
    let inference_runtime_context = agent_inference_runtime_context(1);
    let model_runtime_parameters = DesiredModelRuntimeParameters::default();
    let loaded_llama_model = LoadedLlamaModel::load(
        &inference_runtime_context,
        LlamaModelSettings {
            model_path: smolvlm2_256m()
                .cached_path()
                .expect("the model must be in the Hugging Face cache"),
            placement: ModelPlacement::from(&model_runtime_parameters),
        },
    )
    .expect("the model must be fitted and loaded");
    let llama_context = loaded_llama_model
        .create_llama_context(
            &inference_runtime_context,
            InferenceCapability::TextGeneration,
            loaded_llama_model
                .llama_context_settings(&model_runtime_parameters, 1)
                .into_llama_context_params(),
        )
        .expect("the fitted context must be created");

    assert!(llama_context.n_ctx() >= FitDefaults::of_llama_cpp().n_ctx_min);
}
