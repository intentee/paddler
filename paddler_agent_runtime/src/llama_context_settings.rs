use llama_cpp_bindings::context::params::LlamaContextParams;
use llama_cpp_bindings_sys::LLAMA_FLASH_ATTN_TYPE_AUTO;

use paddler_inference_parameters::model_runtime_parameters::ModelRuntimeParameters;

use crate::agent_kv_cache_dtype::AgentKvCacheDtype;
use crate::converts_to_llama_kv_cache_dtype::ConvertsToLlamaKvCacheDtype as _;

pub struct LlamaContextSettings<'parameters> {
    pub model_runtime_parameters: &'parameters ModelRuntimeParameters,
    pub n_seq_max: u32,
    pub thread_count: i32,
}

impl LlamaContextSettings<'_> {
    #[must_use]
    pub fn into_llama_context_params(self) -> LlamaContextParams {
        let Self {
            model_runtime_parameters,
            n_seq_max,
            thread_count,
        } = self;

        LlamaContextParams::default()
            .with_n_ctx(Some(model_runtime_parameters.context_size))
            .with_n_batch(model_runtime_parameters.n_batch.tokens().get())
            .with_flash_attention_policy(LLAMA_FLASH_ATTN_TYPE_AUTO)
            .with_n_seq_max(n_seq_max)
            .with_n_threads(thread_count)
            .with_n_threads_batch(thread_count)
            .with_type_k(
                AgentKvCacheDtype(model_runtime_parameters.k_cache_dtype.clone())
                    .to_llama_kv_cache_dtype(),
            )
            .with_type_v(
                AgentKvCacheDtype(model_runtime_parameters.v_cache_dtype.clone())
                    .to_llama_kv_cache_dtype(),
            )
    }
}

#[cfg(test)]
mod tests {
    use paddler_inference_parameters::model_runtime_parameters::ModelRuntimeParameters;

    use super::LlamaContextSettings;

    #[test]
    fn carries_the_model_runtime_parameters_into_the_context() {
        let model_runtime_parameters = ModelRuntimeParameters::default();

        let llama_context_params = LlamaContextSettings {
            model_runtime_parameters: &model_runtime_parameters,
            n_seq_max: 4,
            thread_count: 6,
        }
        .into_llama_context_params();

        assert_eq!(
            llama_context_params.n_ctx(),
            Some(model_runtime_parameters.context_size)
        );
        assert_eq!(
            llama_context_params.n_batch(),
            model_runtime_parameters.n_batch.tokens().get()
        );
        assert_eq!(llama_context_params.n_seq_max(), 4);
        assert_eq!(llama_context_params.n_threads(), 6);
        assert_eq!(llama_context_params.n_threads_batch(), 6);
    }
}
