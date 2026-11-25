use std::path::PathBuf;

use llama_cpp_params::llama_model_params::LlamaModelParams;

use paddler_inference_parameters::desired_model_runtime_parameters::DesiredModelRuntimeParameters;
use paddler_inference_parameters::model_default_or::ModelDefaultOr;
use paddler_inference_parameters::offloads_every_layer::offloads_every_layer;
use paddler_messaging::agent_issue_params::model_path::ModelPath;

use crate::model_placement::ModelPlacement;

const LLAMA_CPP_ALL_GPU_LAYERS_KEPT_BY_FITTING: i32 = -2;

const fn llama_cpp_gpu_layers(n_gpu_layers: i32) -> i32 {
    if offloads_every_layer(n_gpu_layers) {
        LLAMA_CPP_ALL_GPU_LAYERS_KEPT_BY_FITTING
    } else {
        n_gpu_layers
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LlamaModelSettings {
    pub model_path: PathBuf,
    pub placement: ModelPlacement,
}

impl LlamaModelSettings {
    #[must_use]
    pub fn reported_model_path(&self) -> ModelPath {
        ModelPath::from(self.model_path.as_path())
    }

    #[must_use]
    pub fn to_llama_model_params(&self) -> LlamaModelParams {
        match &self.placement {
            ModelPlacement::FittedToDeviceMemory(DesiredModelRuntimeParameters {
                n_gpu_layers: ModelDefaultOr::ModelDefault,
                ..
            }) => LlamaModelParams::default(),
            ModelPlacement::FittedToDeviceMemory(DesiredModelRuntimeParameters {
                n_gpu_layers: ModelDefaultOr::Explicit(n_gpu_layers),
                ..
            })
            | ModelPlacement::Offloaded { n_gpu_layers } => {
                LlamaModelParams::default().with_n_gpu_layers(llama_cpp_gpu_layers(*n_gpu_layers))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use llama_cpp_params::llama_model_params::LlamaModelParams;

    use paddler_inference_parameters::constants::ALL_GPU_LAYERS;
    use paddler_inference_parameters::desired_model_runtime_parameters::DesiredModelRuntimeParameters;

    use super::LlamaModelSettings;
    use crate::model_placement::ModelPlacement;

    #[test]
    fn offloads_the_layers_it_was_given() {
        assert_eq!(
            LlamaModelSettings {
                model_path: PathBuf::from("model.gguf"),
                placement: ModelPlacement::Offloaded { n_gpu_layers: 12 },
            }
            .to_llama_model_params()
            .n_gpu_layers(),
            12
        );
    }

    #[test]
    fn all_layers_stay_offloaded_when_llama_cpp_fits_the_model() {
        assert_ne!(
            LlamaModelSettings {
                model_path: PathBuf::from("model.gguf"),
                placement: ModelPlacement::Offloaded {
                    n_gpu_layers: ALL_GPU_LAYERS
                },
            }
            .to_llama_model_params()
            .n_gpu_layers(),
            LlamaModelParams::default().n_gpu_layers()
        );
    }

    #[test]
    fn gpu_layers_following_the_model_are_left_to_llama_cpp() {
        assert_eq!(
            LlamaModelSettings {
                model_path: PathBuf::from("model.gguf"),
                placement: ModelPlacement::FittedToDeviceMemory(
                    DesiredModelRuntimeParameters::default()
                ),
            }
            .to_llama_model_params()
            .n_gpu_layers(),
            LlamaModelParams::default().n_gpu_layers()
        );
    }
}
