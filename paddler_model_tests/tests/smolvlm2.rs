#![cfg(feature = "tests_that_use_llms")]

use std::fs;

use anyhow::Result;
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use paddler_harness::managed_model::ManagedModel;
use paddler_harness::managed_model::ManagedModelParams;
use paddler_harness::model_test_harness::ModelTestHarness;
use paddler_types::conversation_history::ConversationHistory;
use paddler_types::conversation_message::ConversationMessage;
use paddler_types::conversation_message_content::ConversationMessageContent;
use paddler_types::conversation_message_content_part::ConversationMessageContentPart;
use paddler_types::generated_token_result::GeneratedTokenResult;
use paddler_types::huggingface_model_reference::HuggingFaceModelReference;
use paddler_types::image_url::ImageUrl;
use paddler_types::inference_parameters::InferenceParameters;
use paddler_types::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;

fn load_test_image_as_data_uri() -> String {
    let image_bytes = fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/llamas.jpg"
    ))
    .expect("Failed to read test fixture llamas.jpg");

    let encoded = BASE64_STANDARD.encode(&image_bytes);

    format!("data:image/jpeg;base64,{encoded}")
}

#[actix_web::test]
async fn test_smolvlm2_multimodal_inference_with_image() -> Result<()> {
    let managed_model = ManagedModel::from_huggingface(ManagedModelParams {
        inference_parameters: InferenceParameters::default(),
        model: HuggingFaceModelReference {
            filename: "SmolVLM2-256M-Video-Instruct-Q8_0.gguf".to_string(),
            repo_id: "ggml-org/SmolVLM2-256M-Video-Instruct-GGUF".to_string(),
            revision: "main".to_string(),
        },
        multimodal_projection: Some(HuggingFaceModelReference {
            filename: "mmproj-SmolVLM2-256M-Video-Instruct-Q8_0.gguf".to_string(),
            repo_id: "ggml-org/SmolVLM2-256M-Video-Instruct-GGUF".to_string(),
            revision: "main".to_string(),
        }),
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
            max_tokens: 200,
            tools: vec![],
        })
        .await?;

    let token_count = results
        .iter()
        .filter(|result| matches!(result, GeneratedTokenResult::Token(_)))
        .count();

    assert!(
        token_count > 0,
        "Expected to receive at least one token from multimodal inference"
    );
    assert!(
        matches!(results.last(), Some(GeneratedTokenResult::Done)),
        "Expected generation to end with Done"
    );

    managed_model.shutdown()?;

    Ok(())
}
