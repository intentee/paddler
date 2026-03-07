#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use llama_cpp_bindings::LogOptions;
use llama_cpp_bindings::send_logs_to_tracing;
use paddler_harness::managed_model::ManagedModel;
use paddler_harness::managed_model::ManagedModelParams;
use paddler_harness::model_test_harness::ModelTestHarness;
use paddler_model_tests::load_test_image_as_data_uri::load_test_image_as_data_uri;
use paddler_types::conversation_history::ConversationHistory;
use paddler_types::conversation_message::ConversationMessage;
use paddler_types::conversation_message_content::ConversationMessageContent;
use paddler_types::conversation_message_content_part::ConversationMessageContentPart;
use paddler_types::huggingface_model_reference::HuggingFaceModelReference;
use paddler_types::image_url::ImageUrl;
use paddler_types::inference_parameters::InferenceParameters;
use paddler_types::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;

#[actix_web::test]
async fn test_qwen35_rejects_image_input_without_multimodal_projection() -> Result<()> {
    send_logs_to_tracing(LogOptions::default());

    let managed_model = ManagedModel::from_huggingface(ManagedModelParams {
        inference_parameters: InferenceParameters::default(),
        model: HuggingFaceModelReference {
            filename: "Qwen3.5-0.8B-Q4_K_M.gguf".to_string(),
            repo_id: "unsloth/Qwen3.5-0.8B-GGUF".to_string(),
            revision: "main".to_string(),
        },
        multimodal_projection: None,
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
                text: "What do you see?".to_string(),
            },
        ]),
        role: "user".to_string(),
    }]);

    let result = harness
        .generate_from_conversation(ContinueFromConversationHistoryParams {
            add_generation_prompt: true,
            conversation_history,
            enable_thinking: false,
            max_tokens: 100,
            tools: vec![],
        })
        .await;

    assert!(
        result.is_err(),
        "Expected an error when sending images to a text-only model"
    );

    let error_message = result.unwrap_err().to_string();

    assert!(
        error_message.contains("multimodal"),
        "Expected error to mention multimodal, got: {error_message}"
    );

    managed_model.shutdown()?;

    Ok(())
}
