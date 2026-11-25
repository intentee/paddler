use llama_cpp_params::llama_pooling_type::LlamaPoolingType;

pub trait ConvertsToLlamaPoolingType {
    fn to_llama_pooling_type(self) -> LlamaPoolingType;
}
