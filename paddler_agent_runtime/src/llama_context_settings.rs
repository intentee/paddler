use std::num::NonZeroU32;

use llama_cpp_bindings_sys::bindings::LLAMA_FLASH_ATTN_TYPE_AUTO;
use llama_cpp_params::kv_cache_type::KvCacheType as LlamaKvCacheDtype;
use llama_cpp_params::llama_context_params::LlamaContextParams;

use paddler_inference_parameters::batch_size::BatchSize;
use paddler_inference_parameters::desired_model_runtime_parameters::DesiredModelRuntimeParameters;
use paddler_inference_parameters::kv_cache_dtype::KvCacheDtype;
use paddler_inference_parameters::model_default_or::ModelDefaultOr;

use crate::agent_kv_cache_dtype::AgentKvCacheDtype;
use crate::converts_to_llama_kv_cache_dtype::ConvertsToLlamaKvCacheDtype as _;

fn llama_kv_cache_dtype(kv_cache_dtype: KvCacheDtype) -> LlamaKvCacheDtype {
    AgentKvCacheDtype(kv_cache_dtype).to_llama_kv_cache_dtype()
}

pub struct LlamaContextSettings<'parameters> {
    pub context_size: ModelDefaultOr<NonZeroU32>,
    pub model_runtime_parameters: &'parameters DesiredModelRuntimeParameters,
    pub n_seq_max: u32,
    pub thread_count: i32,
}

impl LlamaContextSettings<'_> {
    #[must_use]
    pub fn into_llama_context_params(self) -> LlamaContextParams {
        let Self {
            context_size,
            model_runtime_parameters:
                DesiredModelRuntimeParameters {
                    k_cache_dtype,
                    n_batch,
                    v_cache_dtype,
                    ..
                },
            n_seq_max,
            thread_count,
        } = self;
        let llama_cpp_defaults = LlamaContextParams::default();

        llama_cpp_defaults
            .with_n_ctx(Option::from(context_size))
            .with_n_batch(
                n_batch
                    .map(BatchSize::tokens)
                    .map(NonZeroU32::get)
                    .resolve(llama_cpp_defaults.n_batch()),
            )
            .with_flash_attention_policy(LLAMA_FLASH_ATTN_TYPE_AUTO)
            .with_n_seq_max(n_seq_max)
            .with_n_threads(thread_count)
            .with_n_threads_batch(thread_count)
            .with_type_k(
                k_cache_dtype
                    .clone()
                    .map(llama_kv_cache_dtype)
                    .resolve(llama_cpp_defaults.type_k()),
            )
            .with_type_v(
                v_cache_dtype
                    .clone()
                    .map(llama_kv_cache_dtype)
                    .resolve(llama_cpp_defaults.type_v()),
            )
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU32;

    use llama_cpp_params::kv_cache_type::KvCacheType as LlamaKvCacheDtype;
    use llama_cpp_params::llama_context_params::LlamaContextParams;

    use paddler_inference_parameters::batch_size::BatchSize;
    use paddler_inference_parameters::desired_model_runtime_parameters::DesiredModelRuntimeParameters;
    use paddler_inference_parameters::kv_cache_dtype::KvCacheDtype;
    use paddler_inference_parameters::model_default_or::ModelDefaultOr;

    use super::LlamaContextSettings;

    #[test]
    fn carries_the_explicit_model_runtime_parameters_into_the_context() {
        let llama_context_params = LlamaContextSettings {
            context_size: ModelDefaultOr::Explicit(NonZeroU32::new(4096).unwrap()),
            model_runtime_parameters: &DesiredModelRuntimeParameters {
                k_cache_dtype: ModelDefaultOr::Explicit(KvCacheDtype::F16),
                n_batch: ModelDefaultOr::Explicit(BatchSize::try_from(512).unwrap()),
                v_cache_dtype: ModelDefaultOr::Explicit(KvCacheDtype::Q40),
                ..DesiredModelRuntimeParameters::default()
            },
            n_seq_max: 1,
            thread_count: 1,
        }
        .into_llama_context_params();

        assert_eq!(llama_context_params.n_ctx(), NonZeroU32::new(4096));
        assert_eq!(llama_context_params.n_batch(), 512);
        assert_eq!(llama_context_params.type_k(), LlamaKvCacheDtype::F16);
        assert_eq!(llama_context_params.type_v(), LlamaKvCacheDtype::Q40);
    }

    #[test]
    fn parameters_following_the_model_are_left_to_llama_cpp() {
        let llama_cpp_defaults = LlamaContextParams::default();
        let llama_context_params = LlamaContextSettings {
            context_size: ModelDefaultOr::ModelDefault,
            model_runtime_parameters: &DesiredModelRuntimeParameters::default(),
            n_seq_max: 1,
            thread_count: 1,
        }
        .into_llama_context_params();

        assert_eq!(llama_context_params.n_ctx(), None);
        assert_eq!(llama_context_params.n_batch(), llama_cpp_defaults.n_batch());
        assert_eq!(llama_context_params.type_k(), llama_cpp_defaults.type_k());
        assert_eq!(llama_context_params.type_v(), llama_cpp_defaults.type_v());
    }

    #[test]
    fn carries_the_sequence_and_thread_limits_into_the_context() {
        let llama_context_params = LlamaContextSettings {
            context_size: ModelDefaultOr::ModelDefault,
            model_runtime_parameters: &DesiredModelRuntimeParameters::default(),
            n_seq_max: 4,
            thread_count: 6,
        }
        .into_llama_context_params();

        assert_eq!(llama_context_params.n_seq_max(), 4);
        assert_eq!(llama_context_params.n_threads(), 6);
        assert_eq!(llama_context_params.n_threads_batch(), 6);
    }
}
