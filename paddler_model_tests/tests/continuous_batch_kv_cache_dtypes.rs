#![cfg(feature = "tests_that_use_llms")]
#![expect(
    non_snake_case,
    reason = "test function names embed ggml dtype identifiers (e.g. IQ4_NL) for readable test output and parity with llama.cpp naming"
)]

use anyhow::Result;
use llama_cpp_bindings::LogOptions;
use llama_cpp_bindings::send_logs_to_tracing;
use paddler::agent::continue_from_raw_prompt_request::ContinueFromRawPromptRequest;
use paddler::agent::continuous_batch_scheduler_command::ContinuousBatchSchedulerCommand;
use paddler_model_tests::collect_generated_tokens::collect_generated_tokens;
use paddler_model_tests::managed_model::ManagedModel;
use paddler_model_tests::managed_model_params::ManagedModelParams;
use paddler_types::generated_token_result::GeneratedTokenResult;
use paddler_types::huggingface_model_reference::HuggingFaceModelReference;
use paddler_types::inference_parameters::InferenceParameters;
use paddler_types::kv_cache_dtype::KvCacheDtype;
use paddler_types::request_params::ContinueFromRawPromptParams;
use tokio::sync::mpsc;

async fn assert_generates_tokens_with_kv_cache_dtypes(
    k_cache_dtype: KvCacheDtype,
    v_cache_dtype: KvCacheDtype,
) -> Result<()> {
    send_logs_to_tracing(LogOptions::default());

    let managed_model = ManagedModel::from_huggingface(ManagedModelParams {
        inference_parameters: InferenceParameters {
            k_cache_dtype,
            v_cache_dtype,
            ..InferenceParameters::default()
        },
        model: HuggingFaceModelReference {
            filename: "Qwen3-0.6B-Q8_0.gguf".to_owned(),
            repo_id: "Qwen/Qwen3-0.6B-GGUF".to_owned(),
            revision: "main".to_owned(),
        },
        multimodal_projection: None,
        slots: 1,
    })
    .await?;

    let (generated_tokens_tx, generated_tokens_rx) = mpsc::unbounded_channel();
    let (_stop_tx, generate_tokens_stop_rx) = mpsc::unbounded_channel::<()>();

    managed_model
        .handle()
        .command_tx
        .send(ContinuousBatchSchedulerCommand::ContinueFromRawPrompt(
            ContinueFromRawPromptRequest {
                generated_tokens_tx,
                generate_tokens_stop_rx,
                params: ContinueFromRawPromptParams {
                    grammar: None,
                    max_tokens: 8,
                    raw_prompt: "Count from 1 to 3:".to_owned(),
                },
            },
        ))
        .map_err(|err| anyhow::anyhow!("Failed to send command: {err}"))?;

    let results = collect_generated_tokens(generated_tokens_rx).await?;

    let token_count = results
        .iter()
        .filter(|result| matches!(result, GeneratedTokenResult::Token(_)))
        .count();

    assert!(token_count > 0, "No tokens generated");
    assert!(matches!(results.last(), Some(GeneratedTokenResult::Done)));

    managed_model.shutdown()?;

    Ok(())
}

#[actix_web::test]
async fn generates_tokens_with_F32_kv_cache() -> Result<()> {
    assert_generates_tokens_with_kv_cache_dtypes(KvCacheDtype::F32, KvCacheDtype::F32).await
}

#[actix_web::test]
async fn generates_tokens_with_F16_kv_cache() -> Result<()> {
    assert_generates_tokens_with_kv_cache_dtypes(KvCacheDtype::F16, KvCacheDtype::F16).await
}

#[actix_web::test]
async fn generates_tokens_with_BF16_kv_cache() -> Result<()> {
    assert_generates_tokens_with_kv_cache_dtypes(KvCacheDtype::BF16, KvCacheDtype::BF16).await
}

#[actix_web::test]
async fn generates_tokens_with_Q8_0_kv_cache() -> Result<()> {
    assert_generates_tokens_with_kv_cache_dtypes(KvCacheDtype::Q8_0, KvCacheDtype::Q8_0).await
}

#[actix_web::test]
async fn generates_tokens_with_Q4_0_kv_cache() -> Result<()> {
    assert_generates_tokens_with_kv_cache_dtypes(KvCacheDtype::Q4_0, KvCacheDtype::Q4_0).await
}

#[actix_web::test]
async fn generates_tokens_with_Q4_1_kv_cache() -> Result<()> {
    assert_generates_tokens_with_kv_cache_dtypes(KvCacheDtype::Q4_1, KvCacheDtype::Q4_1).await
}

#[actix_web::test]
async fn generates_tokens_with_IQ4_NL_kv_cache() -> Result<()> {
    assert_generates_tokens_with_kv_cache_dtypes(KvCacheDtype::IQ4_NL, KvCacheDtype::IQ4_NL).await
}

#[actix_web::test]
async fn generates_tokens_with_Q5_0_kv_cache() -> Result<()> {
    assert_generates_tokens_with_kv_cache_dtypes(KvCacheDtype::Q5_0, KvCacheDtype::Q5_0).await
}

#[actix_web::test]
async fn generates_tokens_with_Q5_1_kv_cache() -> Result<()> {
    assert_generates_tokens_with_kv_cache_dtypes(KvCacheDtype::Q5_1, KvCacheDtype::Q5_1).await
}

#[actix_web::test]
async fn generates_tokens_with_distinct_k_and_v_cache_dtypes() -> Result<()> {
    assert_generates_tokens_with_kv_cache_dtypes(KvCacheDtype::Q8_0, KvCacheDtype::Q4_0).await
}
