use std::fs;
use std::time::Duration;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use futures_util::StreamExt;
use integration_tests::BALANCER_INFERENCE_ADDR;
use integration_tests::BALANCER_MANAGEMENT_ADDR;
use integration_tests::balancer_params;
use integration_tests::managed_agent::ManagedAgent;
use integration_tests::managed_agent::ManagedAgentParams;
use integration_tests::managed_balancer::ManagedBalancer;
use paddler_types::agent_desired_model::AgentDesiredModel;
use paddler_types::balancer_desired_state::BalancerDesiredState;
use paddler_types::conversation_history::ConversationHistory;
use paddler_types::conversation_message::ConversationMessage;
use paddler_types::conversation_message_content::ConversationMessageContent;
use paddler_types::conversation_message_content_part::ConversationMessageContentPart;
use paddler_types::huggingface_model_reference::HuggingFaceModelReference;
use paddler_types::image_url::ImageUrl;
use paddler_types::inference_client::Message;
use paddler_types::inference_client::Response;
use paddler_types::inference_parameters::InferenceParameters;
use paddler_types::request_params::ContinueFromConversationHistoryParams;
use paddler_types::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;
use serial_test::file_serial;
use tempfile::NamedTempFile;

fn smolvlm2_model() -> AgentDesiredModel {
    AgentDesiredModel::HuggingFace(HuggingFaceModelReference {
        filename: "SmolVLM2-256M-Video-Instruct-Q8_0.gguf".to_string(),
        repo_id: "ggml-org/SmolVLM2-256M-Video-Instruct-GGUF".to_string(),
        revision: "main".to_string(),
    })
}

fn smolvlm2_mmproj_huggingface() -> AgentDesiredModel {
    AgentDesiredModel::HuggingFace(HuggingFaceModelReference {
        filename: "mmproj-SmolVLM2-256M-Video-Instruct-Q8_0.gguf".to_string(),
        repo_id: "ggml-org/SmolVLM2-256M-Video-Instruct-GGUF".to_string(),
        revision: "main".to_string(),
    })
}

fn load_test_image_as_data_uri() -> String {
    let image_bytes = fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../fixtures/llamas.jpg"
    ))
    .expect("failed to read test fixture llamas.jpg");

    let encoded = BASE64_STANDARD.encode(&image_bytes);

    format!("data:image/jpeg;base64,{encoded}")
}

#[tokio::test]
#[file_serial]
async fn test_load_mmproj_from_local_path() {
    let state_db = NamedTempFile::new().expect("failed to create temp file");

    let desired_state = BalancerDesiredState {
        chat_template_override: None,
        inference_parameters: InferenceParameters::default(),
        model: smolvlm2_model(),
        multimodal_projection: AgentDesiredModel::LocalToAgent("/tmp/test-mmproj.gguf".to_string()),
        use_chat_template_override: false,
    };

    let balancer = ManagedBalancer::spawn(balancer_params(
        BALANCER_MANAGEMENT_ADDR,
        BALANCER_INFERENCE_ADDR,
        state_db.path().to_str().unwrap(),
        10,
        Duration::from_secs(10),
    ))
    .await
    .expect("failed to spawn balancer");

    balancer
        .client()
        .management()
        .put_balancer_desired_state(&desired_state)
        .await
        .expect("failed to set balancer desired state");

    balancer.wait_for_desired_state(&desired_state).await;

    let retrieved_state = balancer
        .client()
        .management()
        .get_balancer_desired_state()
        .await
        .expect("failed to get balancer desired state");

    assert_eq!(
        retrieved_state.multimodal_projection,
        AgentDesiredModel::LocalToAgent("/tmp/test-mmproj.gguf".to_string())
    );
}

#[tokio::test]
#[file_serial]
async fn test_load_mmproj_from_huggingface() {
    let state_db = NamedTempFile::new().expect("failed to create temp file");

    let desired_state = BalancerDesiredState {
        chat_template_override: None,
        inference_parameters: InferenceParameters::default(),
        model: smolvlm2_model(),
        multimodal_projection: smolvlm2_mmproj_huggingface(),
        use_chat_template_override: false,
    };

    let balancer = ManagedBalancer::spawn(balancer_params(
        BALANCER_MANAGEMENT_ADDR,
        BALANCER_INFERENCE_ADDR,
        state_db.path().to_str().unwrap(),
        10,
        Duration::from_secs(10),
    ))
    .await
    .expect("failed to spawn balancer");

    balancer
        .client()
        .management()
        .put_balancer_desired_state(&desired_state)
        .await
        .expect("failed to set balancer desired state");

    balancer.wait_for_desired_state(&desired_state).await;

    let retrieved_state = balancer
        .client()
        .management()
        .get_balancer_desired_state()
        .await
        .expect("failed to get balancer desired state");

    assert_eq!(
        retrieved_state.multimodal_projection,
        smolvlm2_mmproj_huggingface()
    );
}

#[tokio::test]
#[file_serial]
async fn test_multimodal_inference_with_image() {
    let state_db = NamedTempFile::new().expect("failed to create temp file");

    let desired_state = BalancerDesiredState {
        chat_template_override: None,
        inference_parameters: InferenceParameters::default(),
        model: smolvlm2_model(),
        multimodal_projection: smolvlm2_mmproj_huggingface(),
        use_chat_template_override: false,
    };

    let balancer = ManagedBalancer::spawn(balancer_params(
        BALANCER_MANAGEMENT_ADDR,
        BALANCER_INFERENCE_ADDR,
        state_db.path().to_str().unwrap(),
        10,
        Duration::from_secs(10),
    ))
    .await
    .expect("failed to spawn balancer");

    balancer
        .client()
        .management()
        .put_balancer_desired_state(&desired_state)
        .await
        .expect("failed to set balancer desired state");

    balancer.wait_for_desired_state(&desired_state).await;

    let agent = ManagedAgent::spawn(ManagedAgentParams {
        management_addr: BALANCER_MANAGEMENT_ADDR.to_string(),
        name: Some("multimodal-agent".to_string()),
        slots: 4,
    })
    .await
    .expect("failed to spawn agent");

    balancer.wait_for_agent_count(1).await;
    balancer.wait_for_total_slots(4).await;

    let test_image_data_uri = load_test_image_as_data_uri();

    let mut stream = balancer
        .client()
        .inference()
        .continue_from_conversation_history(ContinueFromConversationHistoryParams::<
            ValidatedParametersSchema,
        > {
            add_generation_prompt: true,
            conversation_history: ConversationHistory::new(vec![ConversationMessage {
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
            }]),
            enable_thinking: true,
            max_tokens: 100,
            tools: vec![],
        })
        .await
        .expect("multimodal inference request should succeed");

    let mut received_tokens = false;

    while let Some(message) = stream.next().await {
        let message = message.expect("message should deserialize");

        match message {
            Message::Response(envelope) => match envelope.response {
                Response::GeneratedToken(token_result) => match token_result {
                    paddler_types::generated_token_result::GeneratedTokenResult::Token(_) => {
                        received_tokens = true;
                    }
                    paddler_types::generated_token_result::GeneratedTokenResult::Done => break,
                    other => panic!("unexpected token result: {other:?}"),
                },
                other => panic!("unexpected response: {other:?}"),
            },
            Message::Error(envelope) => {
                panic!(
                    "unexpected error: {} - {}",
                    envelope.error.code, envelope.error.description
                );
            }
        }
    }

    assert!(
        received_tokens,
        "should have received at least one token from multimodal inference"
    );

    drop(agent);
}
