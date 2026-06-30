use core::num::NonZeroU32;

use anyhow::Context as _;
use anyhow::Result;
use llama_cpp_bindings::context::params::LlamaContextParams;
use llama_cpp_bindings_sys::LLAMA_FLASH_ATTN_TYPE_AUTO;
use paddler_messaging::inference_parameters::InferenceParameters;

use crate::agent_kv_cache_dtype::AgentKvCacheDtype;
use crate::agent_pooling_type::AgentPoolingType;
use crate::converts_to_llama_kv_cache_dtype::ConvertsToLlamaKvCacheDtype;
use crate::converts_to_llama_pooling_type::ConvertsToLlamaPoolingType;

pub fn build_inference_context_params(
    inference_parameters: &InferenceParameters,
    n_seq_max: u32,
    n_threads: i32,
    n_threads_batch: i32,
) -> Result<LlamaContextParams> {
    let n_batch =
        u32::try_from(inference_parameters.n_batch).context("n_batch does not fit in u32")?;

    Ok(LlamaContextParams::default()
        .with_embeddings(inference_parameters.enable_embeddings)
        .with_n_ctx(NonZeroU32::new(inference_parameters.context_size))
        .with_n_batch(n_batch)
        .with_flash_attention_policy(LLAMA_FLASH_ATTN_TYPE_AUTO)
        .with_n_seq_max(n_seq_max)
        .with_n_threads(n_threads)
        .with_n_threads_batch(n_threads_batch)
        .with_pooling_type(
            AgentPoolingType(inference_parameters.pooling_type.clone()).to_llama_pooling_type(),
        )
        .with_type_k(
            AgentKvCacheDtype(inference_parameters.k_cache_dtype.clone()).to_llama_kv_cache_dtype(),
        )
        .with_type_v(
            AgentKvCacheDtype(inference_parameters.v_cache_dtype.clone()).to_llama_kv_cache_dtype(),
        ))
}
