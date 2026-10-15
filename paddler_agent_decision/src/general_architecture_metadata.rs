use llama_cpp_bindings::error::MetaValError;
use llama_cpp_bindings::model::LlamaModel;

use crate::reads_model_architecture::ReadsModelArchitecture;

pub struct GeneralArchitectureMetadata;

impl ReadsModelArchitecture for GeneralArchitectureMetadata {
    fn model_architecture(&self, model: &LlamaModel) -> Result<String, MetaValError> {
        model.meta_val_str("general.architecture")
    }
}
