#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use llama_cpp_bindings::LogOptions;
use llama_cpp_bindings::send_logs_to_tracing;
use paddler_model_tests::log_generated_response::log_generated_response;
use paddler_model_tests::managed_model::ManagedModel;
use paddler_model_tests::managed_model::ManagedModelParams;
use paddler_model_tests::model_test_harness::ModelTestHarness;
use paddler_types::generated_token_result::GeneratedTokenResult;
use paddler_types::grammar_constraint::GrammarConstraint;
use paddler_types::huggingface_model_reference::HuggingFaceModelReference;
use paddler_types::inference_parameters::InferenceParameters;
use paddler_types::request_params::ContinueFromRawPromptParams;

fn managed_model_params() -> ManagedModelParams {
    ManagedModelParams {
        inference_parameters: InferenceParameters {
            min_p: 0.0,
            top_k: 20,
            top_p: 0.95,
            ..InferenceParameters::default()
        },
        model: HuggingFaceModelReference {
            filename: "Qwen3-0.6B-Q8_0.gguf".to_string(),
            repo_id: "Qwen/Qwen3-0.6B-GGUF".to_string(),
            revision: "main".to_string(),
        },
        multimodal_projection: None,
    }
}

fn collect_generated_text(results: &[GeneratedTokenResult]) -> String {
    results
        .iter()
        .filter_map(|result| match result {
            GeneratedTokenResult::Token(token) => Some(token.as_str()),
            _ => None,
        })
        .collect()
}

#[actix_web::test]
async fn test_gbnf_grammar_constrains_output() -> Result<()> {
    send_logs_to_tracing(LogOptions::default());

    let managed_model = ManagedModel::from_huggingface(managed_model_params()).await?;
    let harness = ModelTestHarness::new(&managed_model);

    let results = harness
        .generate_from_raw_prompt(ContinueFromRawPromptParams {
            grammar: Some(GrammarConstraint::Gbnf {
                grammar: r#"root ::= "yes" | "no""#.to_owned(),
                root: "root".to_owned(),
            }),
            max_tokens: 10,
            raw_prompt:
                "<|im_start|>user\nIs the sky blue? Answer yes or no.<|im_end|>\n<|im_start|>assistant\n<think>\n\n</think>\n\n"
                    .to_string(),
        })
        .await?;

    log_generated_response(&results);

    let generated_text = collect_generated_text(&results);

    assert!(
        generated_text == "yes" || generated_text == "no",
        "Expected 'yes' or 'no', got: '{generated_text}'"
    );

    managed_model.shutdown()?;

    Ok(())
}

#[actix_web::test]
async fn test_json_schema_grammar_constrains_output() -> Result<()> {
    send_logs_to_tracing(LogOptions::default());

    let managed_model = ManagedModel::from_huggingface(managed_model_params()).await?;
    let harness = ModelTestHarness::new(&managed_model);

    let results = harness
        .generate_from_raw_prompt(ContinueFromRawPromptParams {
            grammar: Some(GrammarConstraint::JsonSchema {
                schema: r#"{"type": "object", "properties": {"answer": {"type": "string"}}, "required": ["answer"]}"#.to_owned(),
            }),
            max_tokens: 50,
            raw_prompt:
                "<|im_start|>user\nWhat is 2+2?<|im_end|>\n<|im_start|>assistant\n<think>\n\n</think>\n\n".to_string(),
        })
        .await?;

    log_generated_response(&results);

    let generated_text = collect_generated_text(&results);
    let parsed: serde_json::Value = serde_json::from_str(&generated_text)?;

    assert!(
        parsed.get("answer").is_some(),
        "Expected JSON with 'answer' field, got: '{generated_text}'"
    );

    managed_model.shutdown()?;

    Ok(())
}

#[actix_web::test]
async fn test_no_grammar_does_not_constrain_output() -> Result<()> {
    send_logs_to_tracing(LogOptions::default());

    let managed_model = ManagedModel::from_huggingface(managed_model_params()).await?;
    let harness = ModelTestHarness::new(&managed_model);

    let results = harness
        .generate_from_raw_prompt(ContinueFromRawPromptParams {
            grammar: None,
            max_tokens: 20,
            raw_prompt: "<|im_start|>user\nSay hello<|im_end|>\n<|im_start|>assistant\n<think>\n\n</think>\n\n"
                .to_string(),
        })
        .await?;

    log_generated_response(&results);

    let token_count = results
        .iter()
        .filter(|result| matches!(result, GeneratedTokenResult::Token(_)))
        .count();

    assert!(
        token_count > 0,
        "Expected to receive at least one token without grammar"
    );

    managed_model.shutdown()?;

    Ok(())
}

#[actix_web::test]
async fn test_grammar_with_thinking_returns_incompatible_error() -> Result<()> {
    use paddler_types::conversation_history::ConversationHistory;
    use paddler_types::conversation_message::ConversationMessage;
    use paddler_types::conversation_message_content::ConversationMessageContent;
    use paddler_types::conversation_message_content_part::ConversationMessageContentPart;
    use paddler_types::request_params::ContinueFromConversationHistoryParams;

    send_logs_to_tracing(LogOptions::default());

    let managed_model = ManagedModel::from_huggingface(managed_model_params()).await?;
    let harness = ModelTestHarness::new(&managed_model);

    let result = harness
        .generate_from_conversation(ContinueFromConversationHistoryParams {
            add_generation_prompt: true,
            enable_thinking: true,
            grammar: Some(GrammarConstraint::JsonSchema {
                schema: r#"{"type": "object", "properties": {"answer": {"type": "string"}}, "required": ["answer"]}"#.to_owned(),
            }),
            conversation_history: ConversationHistory::new(vec![ConversationMessage {
                role: "user".to_owned(),
                content: ConversationMessageContent::Parts(vec![
                    ConversationMessageContentPart::Text {
                        text: "What is 2+2?".to_owned(),
                    },
                ]),
            }]),
            max_tokens: 50,
            tools: vec![],
        })
        .await;

    assert!(
        result.is_err(),
        "Expected error when using grammar with thinking enabled"
    );

    let error_message = format!("{}", result.unwrap_err());

    assert!(
        error_message.contains("grammar constraints and thinking mode cannot be used together"),
        "Expected grammar+thinking incompatibility error, got: '{error_message}'"
    );

    managed_model.shutdown()?;

    Ok(())
}
