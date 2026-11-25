use llama_cpp_bindings_types::model_capabilities::ModelCapabilities;

pub struct ServingReport {
    pub capabilities: ModelCapabilities,
    pub model_path: String,
    pub uses_chat_template_override: bool,
}
