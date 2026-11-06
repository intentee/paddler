use llama_cpp_bindings::context::llama_pooling_type::LlamaPoolingType;

pub trait ConvertsToLlamaPoolingType {
    fn to_llama_pooling_type(self) -> LlamaPoolingType;
}
