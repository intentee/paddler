#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use llama_cpp_bindings::LogOptions;
use llama_cpp_bindings::send_logs_to_tracing;
use paddler_model_tests::load_test_image_as_data_uri::load_test_image_as_data_uri;
use paddler_model_tests::log_generated_response::log_generated_response;
use paddler_model_tests::managed_model::ManagedModel;
use paddler_model_tests::managed_model_params::ManagedModelParams;
use paddler_model_tests::model_test_harness::ModelTestHarness;
use paddler_types::conversation_history::ConversationHistory;
use paddler_types::conversation_message::ConversationMessage;
use paddler_types::conversation_message_content::ConversationMessageContent;
use paddler_types::conversation_message_content_part::ConversationMessageContentPart;
use paddler_types::generated_token_result::GeneratedTokenResult;
use paddler_types::huggingface_model_reference::HuggingFaceModelReference;
use paddler_types::image_url::ImageUrl;
use paddler_types::inference_parameters::InferenceParameters;
use paddler_types::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;

#[actix_web::test]
async fn test_qwen25vl_multimodal_inference_with_image() -> Result<()> {
    send_logs_to_tracing(LogOptions::default());

    let managed_model = ManagedModel::from_huggingface(ManagedModelParams {
        inference_parameters: InferenceParameters::default(),
        model: HuggingFaceModelReference {
            filename: "Qwen2.5-VL-3B-Instruct-Q4_K_M.gguf".to_string(),
            repo_id: "ggml-org/Qwen2.5-VL-3B-Instruct-GGUF".to_string(),
            revision: "main".to_string(),
        },
        multimodal_projection: Some(HuggingFaceModelReference {
            filename: "mmproj-Qwen2.5-VL-3B-Instruct-Q8_0.gguf".to_string(),
            repo_id: "ggml-org/Qwen2.5-VL-3B-Instruct-GGUF".to_string(),
            revision: "main".to_string(),
        }),
        slots: 1,
    })
    .await?;

    let harness = ModelTestHarness::new(&managed_model);

    let test_image_data_uri = load_test_image_as_data_uri();

    let conversation_history = ConversationHistory::new(vec![ConversationMessage {
        content: ConversationMessageContent::Parts(vec![
            ConversationMessageContentPart::ImageUrl {
                image_url: ImageUrl {
                    url: test_image_data_uri,
                },
            },
            ConversationMessageContentPart::Text {
                text: "What do you see in this image?".to_string(),
            },
        ]),
        role: "user".to_string(),
    }]);

    let results = harness
        .generate_from_conversation(ContinueFromConversationHistoryParams {
            add_generation_prompt: true,
            conversation_history,
            enable_thinking: false,
            grammar: None,
            max_tokens: 200,
            tools: vec![],
        })
        .await?;

    log_generated_response(&results);

    let token_count = results
        .iter()
        .filter(|result| matches!(result, GeneratedTokenResult::Token(_)))
        .count();

    assert!(
        token_count > 0,
        "Expected to receive at least one token from Qwen2.5-VL multimodal inference"
    );
    assert!(
        matches!(results.last(), Some(GeneratedTokenResult::Done)),
        "Expected generation to end with Done"
    );

    managed_model.shutdown()?;

    Ok(())
}
