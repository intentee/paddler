use llama_cpp_bindings::context::params::LlamaPoolingType;

pub trait ConvertsToLlamaPoolingType {
    fn to_llama_pooling_type(self) -> LlamaPoolingType;
}
