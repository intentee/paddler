use std::path::PathBuf;

use llama_cpp_bindings::model::params::LlamaModelParams;

use paddler_messaging::agent_issue_params::model_path::ModelPath;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LlamaModelSettings {
    pub model_path: PathBuf,
    pub n_gpu_layers: i32,
}

impl LlamaModelSettings {
    #[must_use]
    pub fn reported_model_path(&self) -> ModelPath {
        ModelPath::from(self.model_path.as_path())
    }

    #[must_use]
    pub fn to_llama_model_params(&self) -> LlamaModelParams {
        LlamaModelParams::default().with_n_gpu_layers(self.n_gpu_layers)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::LlamaModelSettings;

    #[test]
    fn offloads_the_layers_it_was_given() {
        assert_eq!(
            LlamaModelSettings {
                model_path: PathBuf::from("model.gguf"),
                n_gpu_layers: 12,
            }
            .to_llama_model_params()
            .n_gpu_layers(),
            12
        );
    }
}
