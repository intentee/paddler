#![cfg(all(
    feature = "tests_that_use_compiled_paddler",
    feature = "tests_that_use_llms"
))]

use std::fs;
use std::time::Duration;

use anyhow::Context as _;
use anyhow::Result;
use anyhow::anyhow;
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use futures_util::StreamExt;
use paddler_integration_tests::managed_agent::ManagedAgent;
use paddler_integration_tests::managed_agent_params::ManagedAgentParams;
use paddler_integration_tests::managed_balancer::ManagedBalancer;
use paddler_integration_tests::managed_balancer_params::ManagedBalancerParams;
use paddler_integration_tests::managed_cluster::ManagedCluster;
use paddler_integration_tests::managed_cluster_params::ManagedClusterParams;
use paddler_integration_tests::pick_balancer_addresses::pick_balancer_addresses;
use paddler_types::agent_desired_model::AgentDesiredModel;
use paddler_types::balancer_desired_state::BalancerDesiredState;
use paddler_types::conversation_history::ConversationHistory;
use paddler_types::conversation_message::ConversationMessage;
use paddler_types::conversation_message_content::ConversationMessageContent;
use paddler_types::conversation_message_content_part::ConversationMessageContentPart;
use paddler_types::generated_token_result::GeneratedTokenResult;
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
        filename: "SmolVLM2-256M-Video-Instruct-Q8_0.gguf".to_owned(),
        repo_id: "ggml-org/SmolVLM2-256M-Video-Instruct-GGUF".to_owned(),
        revision: "main".to_owned(),
    })
}

fn smolvlm2_mmproj_huggingface() -> AgentDesiredModel {
    AgentDesiredModel::HuggingFace(HuggingFaceModelReference {
        filename: "mmproj-SmolVLM2-256M-Video-Instruct-Q8_0.gguf".to_owned(),
        repo_id: "ggml-org/SmolVLM2-256M-Video-Instruct-GGUF".to_owned(),
        revision: "main".to_owned(),
    })
}

fn load_test_image_as_data_uri() -> Result<String> {
    let image_bytes = fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../fixtures/llamas.jpg"
    ))
    .context("failed to read test fixture llamas.jpg")?;

    let encoded = BASE64_STANDARD.encode(&image_bytes);

    Ok(format!("data:image/jpeg;base64,{encoded}"))
}

#[tokio::test]
#[file_serial]
async fn test_load_mmproj_from_local_path() -> Result<()> {
    let state_db = NamedTempFile::new().context("failed to create temp file")?;
    let state_db_url = format!(
        "file://{}",
        state_db
            .path()
            .to_str()
            .context("temp file path is not valid UTF-8")?
    );
    let addresses = pick_balancer_addresses().context("pick addresses")?;

    let desired_state = BalancerDesiredState {
        chat_template_override: None,
        inference_parameters: InferenceParameters::default(),
        model: smolvlm2_model(),
        multimodal_projection: AgentDesiredModel::LocalToAgent("/tmp/test-mmproj.gguf".to_owned()),
        use_chat_template_override: false,
    };

    let balancer = ManagedBalancer::spawn(ManagedBalancerParams {
        buffered_request_timeout: Duration::from_secs(10),
        compat_openai_addr: addresses.compat_openai.clone(),
        inference_addr: addresses.inference.clone(),
        inference_cors_allowed_hosts: vec![],
        inference_item_timeout: None,
        management_addr: addresses.management.clone(),
        management_cors_allowed_hosts: vec![],
        max_buffered_requests: 10,
        state_database_url: state_db_url.clone(),
    })
    .await
    .context("failed to spawn balancer")?;

    balancer
        .client()
        .management()
        .put_balancer_desired_state(&desired_state)
        .await
        .context("failed to set balancer desired state")?;

    balancer.wait_for_desired_state(&desired_state).await?;

    let retrieved_state = balancer
        .client()
        .management()
        .get_balancer_desired_state()
        .await
        .context("failed to get balancer desired state")?;

    assert_eq!(
        retrieved_state.multimodal_projection,
        AgentDesiredModel::LocalToAgent("/tmp/test-mmproj.gguf".to_owned())
    );

    Ok(())
}

#[tokio::test]
#[file_serial]
async fn test_load_mmproj_from_huggingface() -> Result<()> {
    let state_db = NamedTempFile::new().context("failed to create temp file")?;
    let state_db_url = format!(
        "file://{}",
        state_db
            .path()
            .to_str()
            .context("temp file path is not valid UTF-8")?
    );
    let addresses = pick_balancer_addresses().context("pick addresses")?;

    let desired_state = BalancerDesiredState {
        chat_template_override: None,
        inference_parameters: InferenceParameters::default(),
        model: smolvlm2_model(),
        multimodal_projection: smolvlm2_mmproj_huggingface(),
        use_chat_template_override: false,
    };

    let balancer = ManagedBalancer::spawn(ManagedBalancerParams {
        buffered_request_timeout: Duration::from_secs(10),
        compat_openai_addr: addresses.compat_openai.clone(),
        inference_addr: addresses.inference.clone(),
        inference_cors_allowed_hosts: vec![],
        inference_item_timeout: None,
        management_addr: addresses.management.clone(),
        management_cors_allowed_hosts: vec![],
        max_buffered_requests: 10,
        state_database_url: state_db_url.clone(),
    })
    .await
    .context("failed to spawn balancer")?;

    balancer
        .client()
        .management()
        .put_balancer_desired_state(&desired_state)
        .await
        .context("failed to set balancer desired state")?;

    balancer.wait_for_desired_state(&desired_state).await?;

    let retrieved_state = balancer
        .client()
        .management()
        .get_balancer_desired_state()
        .await
        .context("failed to get balancer desired state")?;

    assert_eq!(
        retrieved_state.multimodal_projection,
        smolvlm2_mmproj_huggingface()
    );

    Ok(())
}

#[tokio::test]
#[file_serial]
async fn test_multimodal_inference_with_image() -> Result<()> {
    let state_db = NamedTempFile::new().context("failed to create temp file")?;
    let state_db_url = format!(
        "file://{}",
        state_db
            .path()
            .to_str()
            .context("temp file path is not valid UTF-8")?
    );
    let addresses = pick_balancer_addresses().context("pick addresses")?;

    let desired_state = BalancerDesiredState {
        chat_template_override: None,
        inference_parameters: InferenceParameters::default(),
        model: smolvlm2_model(),
        multimodal_projection: smolvlm2_mmproj_huggingface(),
        use_chat_template_override: false,
    };

    let balancer = ManagedBalancer::spawn(ManagedBalancerParams {
        buffered_request_timeout: Duration::from_secs(10),
        compat_openai_addr: addresses.compat_openai.clone(),
        inference_addr: addresses.inference.clone(),
        inference_cors_allowed_hosts: vec![],
        inference_item_timeout: None,
        management_addr: addresses.management.clone(),
        management_cors_allowed_hosts: vec![],
        max_buffered_requests: 10,
        state_database_url: state_db_url.clone(),
    })
    .await
    .context("failed to spawn balancer")?;

    balancer
        .client()
        .management()
        .put_balancer_desired_state(&desired_state)
        .await
        .context("failed to set balancer desired state")?;

    balancer.wait_for_desired_state(&desired_state).await?;

    let agent = ManagedAgent::spawn(&ManagedAgentParams {
        management_addr: addresses.management,
        name: Some("multimodal-agent".to_owned()),
        slots: 4,
    })
    .context("failed to spawn agent")?;

    balancer.wait_for_agent_count(1).await?;
    balancer.wait_for_total_slots(4).await?;

    let test_image_data_uri = load_test_image_as_data_uri()?;

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
                        text: "What do you see in this image?".to_owned(),
                    },
                ]),
                role: "user".to_owned(),
            }]),
            enable_thinking: true,
            grammar: None,
            max_tokens: 100,
            tools: vec![],
        })
        .await
        .context("multimodal inference request should succeed")?;

    let mut received_tokens = false;

    while let Some(message) = stream.next().await {
        let message = message.context("message should deserialize")?;

        match message {
            Message::Response(envelope) => match envelope.response {
                Response::GeneratedToken(token_result) => match token_result {
                    GeneratedTokenResult::Token(_) => {
                        received_tokens = true;
                    }
                    GeneratedTokenResult::Done => break,
                    other => return Err(anyhow!("unexpected token result: {other:?}")),
                },
                other => return Err(anyhow!("unexpected response: {other:?}")),
            },
            Message::Error(envelope) => {
                return Err(anyhow!(
                    "unexpected error: {} - {}",
                    envelope.error.code,
                    envelope.error.description
                ));
            }
        }
    }

    assert!(
        received_tokens,
        "should have received at least one token from multimodal inference"
    );

    drop(agent);

    Ok(())
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
                        url: image_data_uri.to_owned(),
                    },
                },
                ConversationMessageContentPart::Text {
                    text: "Describe this image".to_owned(),
                },
            ]),
            role: "user".to_owned(),
        }]),
        enable_thinking: false,
        grammar: None,
        max_tokens: 20,
        tools: vec![],
    }
}

async fn expect_image_decoding_failed(
    cluster: &ManagedCluster,
    image_data_uri: &str,
) -> Result<()> {
    let mut stream = cluster
        .balancer
        .client()
        .inference()
        .continue_from_conversation_history(image_message_params(image_data_uri))
        .await
        .context("request should succeed")?;

    let mut received_decoding_error = false;

    while let Some(message) = stream.next().await {
        let message = message.context("message should deserialize")?;

        match message {
            Message::Response(envelope) => match envelope.response {
                Response::GeneratedToken(token_result) => match token_result {
                    GeneratedTokenResult::ImageDecodingFailed(_) => {
                        received_decoding_error = true;

                        break;
                    }
                    GeneratedTokenResult::Token(_) | GeneratedTokenResult::Done => {}
                    other => return Err(anyhow!("expected ImageDecodingFailed, got {other:?}")),
                },
                other => return Err(anyhow!("unexpected response: {other:?}")),
            },
            Message::Error(envelope) => {
                return Err(anyhow!(
                    "unexpected error: {} - {}",
                    envelope.error.code,
                    envelope.error.description
                ));
            }
        }
    }

    assert!(
        received_decoding_error,
        "should have received ImageDecodingFailed"
    );

    Ok(())
}

#[tokio::test]
#[file_serial]
async fn test_image_sent_to_text_only_model_returns_error() -> Result<()> {
    let cluster = ManagedCluster::spawn(ManagedClusterParams {
        agent_name: "text-only-agent".to_owned(),
        ..ManagedClusterParams::default()
    })
    .await
    .context("failed to spawn cluster")?;

    let test_image_data_uri = load_test_image_as_data_uri()?;

    let mut stream = cluster
        .balancer
        .client()
        .inference()
        .continue_from_conversation_history(image_message_params(&test_image_data_uri))
        .await
        .context("request should succeed")?;

    let mut received_error = false;

    while let Some(message) = stream.next().await {
        let message = message.context("message should deserialize")?;

        match message {
            Message::Response(envelope) => match envelope.response {
                Response::GeneratedToken(token_result) => match token_result {
                    GeneratedTokenResult::ChatTemplateError(_)
                    | GeneratedTokenResult::MultimodalNotSupported(_) => {
                        received_error = true;

                        break;
                    }
                    GeneratedTokenResult::Token(_) | GeneratedTokenResult::Done => {}
                    other => {
                        return Err(anyhow!(
                            "expected error for image on text-only model, got {other:?}"
                        ));
                    }
                },
                other => return Err(anyhow!("unexpected response: {other:?}")),
            },
            Message::Error(envelope) => {
                return Err(anyhow!(
                    "unexpected error: {} - {}",
                    envelope.error.code,
                    envelope.error.description
                ));
            }
        }
    }

    assert!(received_error, "text-only model should reject image input");

    Ok(())
}

#[tokio::test]
#[file_serial]
async fn test_malformed_data_uri_returns_error() -> Result<()> {
    let cluster = ManagedCluster::spawn(ManagedClusterParams {
        agent_name: "text-only-agent".to_owned(),
        ..ManagedClusterParams::default()
    })
    .await
    .context("failed to spawn cluster")?;

    expect_image_decoding_failed(&cluster, "data:image/jpegbase64,abc123").await?;

    Ok(())
}

#[tokio::test]
#[file_serial]
async fn test_invalid_base64_returns_error() -> Result<()> {
    let cluster = ManagedCluster::spawn(ManagedClusterParams {
        agent_name: "text-only-agent".to_owned(),
        ..ManagedClusterParams::default()
    })
    .await
    .context("failed to spawn cluster")?;

    expect_image_decoding_failed(&cluster, "data:image/jpeg;base64,!!!not-valid-base64!!!").await?;

    Ok(())
}

#[tokio::test]
#[file_serial]
async fn test_remote_url_returns_error() -> Result<()> {
    let cluster = ManagedCluster::spawn(ManagedClusterParams {
        agent_name: "text-only-agent".to_owned(),
        ..ManagedClusterParams::default()
    })
    .await
    .context("failed to spawn cluster")?;

    expect_image_decoding_failed(&cluster, "https://example.com/image.jpg").await?;

    Ok(())
}
