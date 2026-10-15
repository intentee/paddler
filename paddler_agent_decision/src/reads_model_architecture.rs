use llama_cpp_bindings::error::MetaValError;
use llama_cpp_bindings::model::LlamaModel;

pub trait ReadsModelArchitecture: Send + 'static {
    fn model_architecture(&self, model: &LlamaModel) -> Result<String, MetaValError>;
}
