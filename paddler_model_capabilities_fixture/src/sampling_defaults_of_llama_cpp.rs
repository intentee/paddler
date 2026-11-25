use llama_cpp_bindings_types::model_defaults::ModelDefaults;
use llama_cpp_bindings_types::sampling_defaults::SamplingDefaults;

#[must_use]
pub const fn model_defaults_of_llama_cpp() -> ModelDefaults {
    ModelDefaults {
        context_size: 4096,
        sampling: SamplingDefaults {
            min_p: 0.05,
            penalty_frequency: 0.0,
            penalty_last_n: 64,
            penalty_presence: 0.0,
            penalty_repeat: 1.0,
            temperature: 0.8,
            top_k: 40,
            top_p: 0.95,
        },
    }
}
