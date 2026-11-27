#![cfg(feature = "tests_that_use_llms")]

use std::path::PathBuf;

use llama_cpp_params::params_error::ParamsError;

use paddler_agent_pipeline_harness::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_agent_runtime::agent_runtime_error::AgentRuntimeError;
use paddler_agent_runtime::llama_model_settings::LlamaModelSettings;
use paddler_agent_runtime::loaded_llama_model::LoadedLlamaModel;
use paddler_agent_runtime::model_placement::ModelPlacement;
use paddler_inference_parameters::desired_model_runtime_parameters::DesiredModelRuntimeParameters;

#[test]
fn a_model_path_llama_cpp_cannot_receive_fails_its_fit() {
    let model_path_with_a_nul_byte = PathBuf::from("/paddler/model\0.gguf");

    assert!(matches!(
        LoadedLlamaModel::load(
            &agent_inference_runtime_context(1),
            LlamaModelSettings {
                model_path: model_path_with_a_nul_byte.clone(),
                placement: ModelPlacement::FittedToDeviceMemory(
                    DesiredModelRuntimeParameters::default()
                ),
            },
        ),
        Err(AgentRuntimeError::ModelFitFailed {
            model_path,
            source: ParamsError::FitModelPathInvalid(_path_conversion_error),
        }) if model_path == model_path_with_a_nul_byte
    ));
}
