#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use llama_cpp_bindings::LogOptions;
use llama_cpp_bindings::send_logs_to_tracing;
use paddler_model_tests::managed_model::ManagedModel;
use paddler_model_tests::managed_model_params::ManagedModelParams;
use paddler_model_tests::model_test_harness::ModelTestHarness;
use paddler_types::generated_token_result::GeneratedTokenResult;
use paddler_types::huggingface_model_reference::HuggingFaceModelReference;
use paddler_types::inference_parameters::InferenceParameters;
use paddler_types::request_params::ContinueFromRawPromptParams;

#[actix_web::test]
async fn test_slot_reused_after_first_request_completes() -> Result<()> {
    send_logs_to_tracing(LogOptions::default());

    let managed_model = ManagedModel::from_huggingface(ManagedModelParams {
        inference_parameters: InferenceParameters::default(),
        model: HuggingFaceModelReference {
            filename: "Qwen3-0.6B-Q8_0.gguf".to_string(),
            repo_id: "Qwen/Qwen3-0.6B-GGUF".to_string(),
            revision: "main".to_string(),
        },
        multimodal_projection: None,
        slots: 1,
    })
    .await?;

    let harness = ModelTestHarness::new(&managed_model);

    // First request
    let results_1 = harness
        .generate_from_raw_prompt(ContinueFromRawPromptParams {
            grammar: None,
            max_tokens: 10,
            raw_prompt: "Hello world".to_string(),
        })
        .await?;

    let token_count_1 = results_1
        .iter()
        .filter(|result| matches!(result, GeneratedTokenResult::Token(_)))
        .count();

    assert!(token_count_1 > 0, "First request should produce tokens");
    assert!(
        matches!(results_1.last(), Some(GeneratedTokenResult::Done)),
        "First request should end with Done"
    );

    // Second request: reuses the same slot
    let results_2 = harness
        .generate_from_raw_prompt(ContinueFromRawPromptParams {
            grammar: None,
            max_tokens: 10,
            raw_prompt: "Goodbye world".to_string(),
        })
        .await?;

    let token_count_2 = results_2
        .iter()
        .filter(|result| matches!(result, GeneratedTokenResult::Token(_)))
        .count();

    assert!(token_count_2 > 0, "Second request should produce tokens");
    assert!(
        matches!(results_2.last(), Some(GeneratedTokenResult::Done)),
        "Second request should end with Done"
    );

    managed_model.shutdown()?;

    Ok(())
}
