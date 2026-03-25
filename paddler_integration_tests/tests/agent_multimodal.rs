#![cfg(feature = "tests_that_use_compiled_paddler")]

use std::fs;
use std::time::Duration;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use futures_util::StreamExt;
use paddler_integration_tests::BALANCER_INFERENCE_ADDR;
use paddler_integration_tests::BALANCER_MANAGEMENT_ADDR;
use paddler_integration_tests::BALANCER_OPENAI_ADDR;
use paddler_integration_tests::managed_agent::ManagedAgent;
use paddler_integration_tests::managed_agent::ManagedAgentParams;
use paddler_integration_tests::managed_balancer::ManagedBalancer;
use paddler_integration_tests::managed_balancer::ManagedBalancerParams;
use paddler_integration_tests::managed_cluster::ManagedCluster;
use paddler_integration_tests::managed_cluster_params::ManagedClusterParams;
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
    let state_db_url = format!("file://{}", state_db.path().to_str().unwrap());

    let desired_state = BalancerDesiredState {
        chat_template_override: None,
        inference_parameters: InferenceParameters::default(),
        model: smolvlm2_model(),
        multimodal_projection: AgentDesiredModel::LocalToAgent("/tmp/test-mmproj.gguf".to_string()),
        use_chat_template_override: false,
    };

    let balancer = ManagedBalancer::spawn(ManagedBalancerParams {
        buffered_request_timeout: Duration::from_secs(10),
        compat_openai_addr: BALANCER_OPENAI_ADDR.to_owned(),
        inference_addr: BALANCER_INFERENCE_ADDR.to_owned(),
        inference_cors_allowed_hosts: vec![],
        inference_item_timeout: None,
        management_addr: BALANCER_MANAGEMENT_ADDR.to_owned(),
        management_cors_allowed_hosts: vec![],
        max_buffered_requests: 10,
        state_database_url: state_db_url.to_owned(),
    })
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
    let state_db_url = format!("file://{}", state_db.path().to_str().unwrap());

    let desired_state = BalancerDesiredState {
        chat_template_override: None,
        inference_parameters: InferenceParameters::default(),
        model: smolvlm2_model(),
        multimodal_projection: smolvlm2_mmproj_huggingface(),
        use_chat_template_override: false,
    };

    let balancer = ManagedBalancer::spawn(ManagedBalancerParams {
        buffered_request_timeout: Duration::from_secs(10),
        compat_openai_addr: BALANCER_OPENAI_ADDR.to_owned(),
        inference_addr: BALANCER_INFERENCE_ADDR.to_owned(),
        inference_cors_allowed_hosts: vec![],
        inference_item_timeout: None,
        management_addr: BALANCER_MANAGEMENT_ADDR.to_owned(),
        management_cors_allowed_hosts: vec![],
        max_buffered_requests: 10,
        state_database_url: state_db_url.to_owned(),
    })
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
    let state_db_url = format!("file://{}", state_db.path().to_str().unwrap());

    let desired_state = BalancerDesiredState {
        chat_template_override: None,
        inference_parameters: InferenceParameters::default(),
        model: smolvlm2_model(),
        multimodal_projection: smolvlm2_mmproj_huggingface(),
        use_chat_template_override: false,
    };

    let balancer = ManagedBalancer::spawn(ManagedBalancerParams {
        buffered_request_timeout: Duration::from_secs(10),
        compat_openai_addr: BALANCER_OPENAI_ADDR.to_owned(),
        inference_addr: BALANCER_INFERENCE_ADDR.to_owned(),
        inference_cors_allowed_hosts: vec![],
        inference_item_timeout: None,
        management_addr: BALANCER_MANAGEMENT_ADDR.to_owned(),
        management_cors_allowed_hosts: vec![],
        max_buffered_requests: 10,
        state_database_url: state_db_url.to_owned(),
    })
    .await
    .expect("failed to spawn balancer");

    balancer
        .client()
        .management()
        .put_balancer_desired_state(&desired_state)
        .await
        .expect("failed to set balancer desired state");

    balancer.wait_for_desired_state(&desired_state).await;

    let agent = ManagedAgent::spawn(&ManagedAgentParams {
        management_addr: BALANCER_MANAGEMENT_ADDR.to_string(),
        name: Some("multimodal-agent".to_string()),
        slots: 4,
    })
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

fn image_message_params(
    image_data_uri: &str,
) -> ContinueFromConversationHistoryParams<ValidatedParametersSchema> {
    ContinueFromConversationHistoryParams {
        add_generation_prompt: true,
        conversation_history: ConversationHistory::new(vec![ConversationMessage {
            content: ConversationMessageContent::Parts(vec![
                ConversationMessageContentPart::ImageUrl {
                    image_url: ImageUrl {
                        url: image_data_uri.to_string(),
                    },
                },
                ConversationMessageContentPart::Text {
                    text: "Describe this image".to_string(),
                },
            ]),
            role: "user".to_string(),
        }]),
        enable_thinking: false,
        max_tokens: 20,
        tools: vec![],
    }
}

async fn expect_image_decoding_failed(cluster: &ManagedCluster, image_data_uri: &str) {
    let mut stream = cluster
        .balancer
        .client()
        .inference()
        .continue_from_conversation_history(image_message_params(image_data_uri))
        .await
        .expect("request should succeed");

    let mut received_decoding_error = false;

    while let Some(message) = stream.next().await {
        let message = message.expect("message should deserialize");

        match message {
            Message::Response(envelope) => match envelope.response {
                Response::GeneratedToken(token_result) => match token_result {
                    paddler_types::generated_token_result::GeneratedTokenResult::ImageDecodingFailed(_) => {
                        received_decoding_error = true;

                        break;
                    }
                    paddler_types::generated_token_result::GeneratedTokenResult::Token(_)
                    | paddler_types::generated_token_result::GeneratedTokenResult::Done => continue,
                    other => panic!("expected ImageDecodingFailed, got {other:?}"),
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
        received_decoding_error,
        "should have received ImageDecodingFailed"
    );
}

#[tokio::test]
#[file_serial]
async fn test_image_sent_to_text_only_model_returns_error() {
    let cluster = ManagedCluster::spawn(ManagedClusterParams {
        agent_name: "text-only-agent".to_string(),
        ..ManagedClusterParams::default()
    })
    .await
    .expect("failed to spawn cluster");

    let test_image_data_uri = load_test_image_as_data_uri();

    let mut stream = cluster
        .balancer
        .client()
        .inference()
        .continue_from_conversation_history(image_message_params(&test_image_data_uri))
        .await
        .expect("request should succeed");

    let mut received_error = false;

    while let Some(message) = stream.next().await {
        let message = message.expect("message should deserialize");

        match message {
            Message::Response(envelope) => match envelope.response {
                Response::GeneratedToken(token_result) => match token_result {
                    paddler_types::generated_token_result::GeneratedTokenResult::ChatTemplateError(_)
                    | paddler_types::generated_token_result::GeneratedTokenResult::MultimodalNotSupported(_) => {
                        received_error = true;

                        break;
                    }
                    paddler_types::generated_token_result::GeneratedTokenResult::Token(_)
                    | paddler_types::generated_token_result::GeneratedTokenResult::Done => continue,
                    other => panic!("expected error for image on text-only model, got {other:?}"),
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

    assert!(received_error, "text-only model should reject image input");
}

#[tokio::test]
#[file_serial]
async fn test_malformed_data_uri_returns_error() {
    let cluster = ManagedCluster::spawn(ManagedClusterParams {
        agent_name: "text-only-agent".to_string(),
        ..ManagedClusterParams::default()
    })
    .await
    .expect("failed to spawn cluster");

    expect_image_decoding_failed(&cluster, "data:image/jpegbase64,abc123").await;
}

#[tokio::test]
#[file_serial]
async fn test_invalid_base64_returns_error() {
    let cluster = ManagedCluster::spawn(ManagedClusterParams {
        agent_name: "text-only-agent".to_string(),
        ..ManagedClusterParams::default()
    })
    .await
    .expect("failed to spawn cluster");

    expect_image_decoding_failed(&cluster, "data:image/jpeg;base64,!!!not-valid-base64!!!").await;
}

#[tokio::test]
#[file_serial]
async fn test_remote_url_returns_error() {
    let cluster = ManagedCluster::spawn(ManagedClusterParams {
        agent_name: "text-only-agent".to_string(),
        ..ManagedClusterParams::default()
    })
    .await
    .expect("failed to spawn cluster");

    expect_image_decoding_failed(&cluster, "https://example.com/image.jpg").await;
}
