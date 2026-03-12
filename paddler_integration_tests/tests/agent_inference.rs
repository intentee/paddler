use std::time::Duration;

use futures_util::StreamExt;
use integration_tests::AGENT_DESIRED_MODEL;
use integration_tests::BALANCER_INFERENCE_ADDR;
use integration_tests::BALANCER_MANAGEMENT_ADDR;
use integration_tests::BALANCER_OPENAI_ADDR;
use integration_tests::balancer_params;
use integration_tests::balancer_params_with_openai;
use integration_tests::managed_agent::ManagedAgent;
use integration_tests::managed_agent::ManagedAgentParams;
use integration_tests::managed_balancer::ManagedBalancer;
use paddler_types::agent_desired_model::AgentDesiredModel;
use paddler_types::balancer_desired_state::BalancerDesiredState;
use paddler_types::conversation_history::ConversationHistory;
use paddler_types::conversation_message::ConversationMessage;
use paddler_types::conversation_message_content::ConversationMessageContent;
use paddler_types::inference_client::Message;
use paddler_types::inference_client::Response;
use paddler_types::inference_parameters::InferenceParameters;
use paddler_types::request_params::ContinueFromConversationHistoryParams;
use paddler_types::request_params::ContinueFromRawPromptParams;
use paddler_types::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;
use serial_test::file_serial;
use tempfile::NamedTempFile;

struct InferenceTestCluster {
    balancer: ManagedBalancer,
    _agent: ManagedAgent,
    _state_db: NamedTempFile,
}

async fn spawn_inference_cluster() -> InferenceTestCluster {
    let state_db = NamedTempFile::new().expect("failed to create temp file");

    let desired_state = BalancerDesiredState {
        chat_template_override: None,
        inference_parameters: InferenceParameters::default(),
        model: AGENT_DESIRED_MODEL.clone(),
        multimodal_projection: AgentDesiredModel::None,
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
        name: Some("inference-agent".to_string()),
        slots: 4,
    })
    .await
    .expect("failed to spawn agent");

    balancer.wait_for_agent_count(1).await;
    balancer.wait_for_total_slots(4).await;

    InferenceTestCluster {
        balancer,
        _agent: agent,
        _state_db: state_db,
    }
}

struct OpenAITestCluster {
    _balancer: ManagedBalancer,
    openai_base_url: String,
    _agent: ManagedAgent,
    _state_db: NamedTempFile,
}

async fn spawn_openai_cluster() -> OpenAITestCluster {
    let state_db = NamedTempFile::new().expect("failed to create temp file");

    let desired_state = BalancerDesiredState {
        chat_template_override: None,
        inference_parameters: InferenceParameters::default(),
        model: AGENT_DESIRED_MODEL.clone(),
        multimodal_projection: AgentDesiredModel::None,
        use_chat_template_override: false,
    };

    let balancer = ManagedBalancer::spawn(balancer_params_with_openai(
        BALANCER_MANAGEMENT_ADDR,
        BALANCER_INFERENCE_ADDR,
        BALANCER_OPENAI_ADDR,
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
        name: Some("openai-agent".to_string()),
        slots: 4,
    })
    .await
    .expect("failed to spawn agent");

    balancer.wait_for_agent_count(1).await;
    balancer.wait_for_total_slots(4).await;

    let openai_base_url = format!("http://{BALANCER_OPENAI_ADDR}");

    OpenAITestCluster {
        _balancer: balancer,
        openai_base_url,
        _agent: agent,
        _state_db: state_db,
    }
}

#[tokio::test]
#[file_serial]
async fn test_inference_health_endpoint() {
    let state_db = NamedTempFile::new().expect("failed to create temp file");

    let balancer = ManagedBalancer::spawn(balancer_params(
        BALANCER_MANAGEMENT_ADDR,
        BALANCER_INFERENCE_ADDR,
        state_db.path().to_str().unwrap(),
        10,
        Duration::from_secs(10),
    ))
    .await
    .expect("failed to spawn balancer");

    let response = balancer
        .client()
        .inference()
        .get_health()
        .await
        .expect("health request should succeed");

    assert_eq!(response, "OK");
}

#[tokio::test]
#[file_serial]
async fn test_continue_from_raw_prompt() {
    let cluster = spawn_inference_cluster().await;

    let mut stream = cluster
        .balancer
        .client()
        .inference()
        .continue_from_raw_prompt(ContinueFromRawPromptParams {
            max_tokens: 10,
            raw_prompt: "The capital of France is".to_string(),
        })
        .await
        .expect("raw prompt request should succeed");

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

    assert!(received_tokens, "should have received at least one token");
}

#[tokio::test]
#[file_serial]
async fn test_continue_from_conversation_history() {
    let _cluster = spawn_inference_cluster().await;

    let params = ContinueFromConversationHistoryParams::<ValidatedParametersSchema> {
        add_generation_prompt: true,
        conversation_history: ConversationHistory::new(vec![ConversationMessage {
            content: ConversationMessageContent::Text("Say hello".to_string()),
            role: "user".to_string(),
        }]),
        enable_thinking: true,
        max_tokens: 50,
        tools: vec![],
    };

    let http_client = reqwest::Client::new();

    let response = http_client
        .post(format!(
            "http://{BALANCER_INFERENCE_ADDR}/api/v1/continue_from_conversation_history"
        ))
        .json(&params)
        .send()
        .await
        .expect("conversation history request should succeed");

    assert_eq!(response.status(), 200);

    let body = response.text().await.expect("should read response body");
    let mut received_tokens = false;

    for line in body.lines() {
        let line = line.trim();

        if line.is_empty() {
            continue;
        }

        let message: Message =
            serde_json::from_str(line).expect("each line should be valid inference message JSON");

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

    assert!(received_tokens, "should have received at least one token");
}

#[tokio::test]
#[file_serial]
async fn test_openai_chat_completions_non_streaming() {
    let cluster = spawn_openai_cluster().await;

    let http_client = reqwest::Client::new();

    let response = http_client
        .post(format!("{}/v1/chat/completions", cluster.openai_base_url))
        .json(&serde_json::json!({
            "model": "test",
            "messages": [
                {
                    "role": "user",
                    "content": "Say hello"
                }
            ],
            "max_completion_tokens": 10,
            "stream": false
        }))
        .send()
        .await
        .expect("openai request should succeed");

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.expect("should parse json response");

    assert_eq!(body["object"], "chat.completion");
    assert!(body["choices"].is_array(), "should have choices array");
    assert!(
        !body["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .is_empty(),
        "response content should not be empty"
    );
}

#[tokio::test]
#[file_serial]
async fn test_openai_chat_completions_streaming() {
    let cluster = spawn_openai_cluster().await;

    let http_client = reqwest::Client::new();

    let response = http_client
        .post(format!("{}/v1/chat/completions", cluster.openai_base_url))
        .json(&serde_json::json!({
            "model": "test",
            "messages": [
                {
                    "role": "user",
                    "content": "Say hello"
                }
            ],
            "max_completion_tokens": 10,
            "stream": true
        }))
        .send()
        .await
        .expect("openai streaming request should succeed");

    assert_eq!(response.status(), 200);

    let body = response.text().await.expect("should read response body");
    let chunks: Vec<serde_json::Value> = body
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).expect("each line should be valid JSON"))
        .collect();

    assert!(!chunks.is_empty(), "should have received streaming chunks");
    assert_eq!(chunks[0]["object"], "chat.completion.chunk");
}

#[tokio::test]
#[file_serial]
async fn test_openai_health_endpoint() {
    let state_db = NamedTempFile::new().expect("failed to create temp file");

    let _balancer = ManagedBalancer::spawn(balancer_params_with_openai(
        BALANCER_MANAGEMENT_ADDR,
        BALANCER_INFERENCE_ADDR,
        BALANCER_OPENAI_ADDR,
        state_db.path().to_str().unwrap(),
        10,
        Duration::from_secs(10),
    ))
    .await
    .expect("failed to spawn balancer");

    let response = reqwest::get(format!("http://{BALANCER_OPENAI_ADDR}/health"))
        .await
        .expect("openai health request should succeed");

    assert_eq!(response.status(), 200);
    assert_eq!(response.text().await.expect("should read body"), "OK");
}
