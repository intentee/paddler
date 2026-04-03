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
async fn test_generation_stops_at_max_tokens_boundary() -> Result<()> {
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

    let results = harness
        .generate_from_raw_prompt(ContinueFromRawPromptParams {
            grammar: None,
            max_tokens: 5,
            raw_prompt: "Count from one to one hundred:".to_string(),
        })
        .await?;

    let token_count = results
        .iter()
        .filter(|result| matches!(result, GeneratedTokenResult::Token(_)))
        .count();

    assert_eq!(
        token_count, 5,
        "Expected exactly 5 tokens with max_tokens=5, got {token_count}"
    );

    assert!(
        matches!(results.last(), Some(GeneratedTokenResult::Done)),
        "Expected generation to end with Done"
    );

    managed_model.shutdown()?;

    Ok(())
}
