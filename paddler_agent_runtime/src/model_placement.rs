use paddler_inference_parameters::desired_model_runtime_parameters::DesiredModelRuntimeParameters;
use paddler_inference_parameters::model_default_or::ModelDefaultOr;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ModelPlacement {
    FittedToDeviceMemory(DesiredModelRuntimeParameters),
    Offloaded { n_gpu_layers: i32 },
}

impl From<&DesiredModelRuntimeParameters> for ModelPlacement {
    fn from(model_runtime_parameters: &DesiredModelRuntimeParameters) -> Self {
        match model_runtime_parameters {
            DesiredModelRuntimeParameters {
                context_size: ModelDefaultOr::Explicit(_),
                n_gpu_layers: ModelDefaultOr::Explicit(n_gpu_layers),
                ..
            } => Self::Offloaded {
                n_gpu_layers: *n_gpu_layers,
            },
            DesiredModelRuntimeParameters { .. } => {
                Self::FittedToDeviceMemory(model_runtime_parameters.clone())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU32;

    use paddler_inference_parameters::desired_model_runtime_parameters::DesiredModelRuntimeParameters;
    use paddler_inference_parameters::model_default_or::ModelDefaultOr;

    use super::ModelPlacement;

    #[test]
    fn an_explicit_context_and_explicit_gpu_layers_offload_the_model_as_asked() {
        assert_eq!(
            ModelPlacement::from(&DesiredModelRuntimeParameters {
                context_size: ModelDefaultOr::Explicit(NonZeroU32::new(4096).unwrap()),
                n_gpu_layers: ModelDefaultOr::Explicit(12),
                ..DesiredModelRuntimeParameters::default()
            }),
            ModelPlacement::Offloaded { n_gpu_layers: 12 }
        );
    }

    #[test]
    fn a_parameter_following_the_model_fits_the_model_to_device_memory() {
        let model_runtime_parameters = DesiredModelRuntimeParameters {
            n_gpu_layers: ModelDefaultOr::Explicit(12),
            ..DesiredModelRuntimeParameters::default()
        };

        assert_eq!(
            ModelPlacement::from(&model_runtime_parameters),
            ModelPlacement::FittedToDeviceMemory(model_runtime_parameters)
        );
    }
}
