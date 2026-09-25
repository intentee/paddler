#![cfg(feature = "tests_that_use_llms")]

use std::mem::discriminant;
use std::sync::Arc;

use anyhow::Result;
use llama_cpp_bindings::llama_backend::LlamaBackend;
use paddler_agent::continue_from_raw_prompt_request::ContinueFromRawPromptRequest;
use paddler_agent::generation_request_rejection::GenerationRequestRejection;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_tests::detached_slot_guard::detached_slot_guard;
use paddler_tests::embeddings_mode_generation_request_preparer::embeddings_mode_generation_request_preparer;
use paddler_tests::load_model_from_card::load_model_from_card;
use paddler_tests::model_card::qwen3_0_6b::qwen3_0_6b;
use tokio::sync::mpsc;

#[test]
fn generation_request_preparer_rejects_raw_prompt_when_token_generation_is_disabled() -> Result<()>
{
    let llama_backend = LlamaBackend::init()?;
    let preparer = embeddings_mode_generation_request_preparer(Arc::new(load_model_from_card(
        &llama_backend,
        qwen3_0_6b(),
    )?));
    let (generated_tokens_tx, _generated_tokens_rx) = mpsc::unbounded_channel();
    let (_generate_tokens_stop_tx, generate_tokens_stop_rx) = mpsc::unbounded_channel();

    let rejection = preparer
        .prepare_raw_prompt(ContinueFromRawPromptRequest {
            generate_tokens_stop_rx,
            generated_tokens_tx,
            params: ContinueFromRawPromptParams {
                grammar: None,
                max_tokens: 1,
                raw_prompt: "Hello".to_owned(),
            },
            slot_guard: detached_slot_guard(),
        })
        .err()
        .map(|rejection| discriminant(&rejection));

    assert_eq!(
        rejection,
        Some(discriminant(
            &GenerationRequestRejection::TokenGenerationDisabled
        ))
    );

    Ok(())
}
