use std::time::Duration;

use futures_util::StreamExt;
use integration_tests::AGENT_DESIRED_MODEL;
use integration_tests::BALANCER_INFERENCE_ADDR;
use integration_tests::BALANCER_MANAGEMENT_ADDR;
use integration_tests::balancer_params;
use integration_tests::managed_agent::ManagedAgent;
use integration_tests::managed_agent::ManagedAgentParams;
use integration_tests::managed_balancer::ManagedBalancer;
use paddler_types::agent_desired_model::AgentDesiredModel;
use paddler_types::balancer_desired_state::BalancerDesiredState;
use paddler_types::inference_parameters::InferenceParameters;
use paddler_types::request_params::ContinueFromRawPromptParams;
use serial_test::file_serial;
use tempfile::NamedTempFile;

#[tokio::test]
#[file_serial]
async fn test_health_endpoint_returns_ok() {
    let state_db = NamedTempFile::new().expect("failed to create temp file");

    let balancer = ManagedBalancer::spawn(balancer_params(
        BALANCER_MANAGEMENT_ADDR,
        BALANCER_INFERENCE_ADDR,
        state_db.path().to_str().unwrap(),
        30,
        Duration::from_secs(10),
    ))
    .await
    .expect("failed to spawn balancer");

    let health = balancer
        .client()
        .management()
        .get_health()
        .await
        .expect("health endpoint should respond");

    assert_eq!(health, "OK");
}

#[tokio::test]
#[file_serial]
async fn test_inference_fails_when_no_model_configured() {
    let state_db = NamedTempFile::new().expect("failed to create temp file");

    let balancer = ManagedBalancer::spawn(balancer_params(
        BALANCER_MANAGEMENT_ADDR,
        BALANCER_INFERENCE_ADDR,
        state_db.path().to_str().unwrap(),
        30,
        Duration::from_secs(10),
    ))
    .await
    .expect("failed to spawn balancer");

    let result = balancer
        .client()
        .inference()
        .continue_from_raw_prompt(ContinueFromRawPromptParams {
            max_tokens: 10,
            raw_prompt: "Hello".to_string(),
        })
        .await;

    assert!(result.is_ok(), "WebSocket connection should succeed");

    let mut stream = result.unwrap();
    let first_message = stream.next().await;

    assert!(first_message.is_some(), "should receive a response message");

    let message = first_message.unwrap().expect("message should deserialize");

    match message {
        paddler_types::inference_client::Message::Error(envelope) => {
            assert_eq!(envelope.error.code, 504);
            assert_eq!(
                envelope.error.description,
                "Waiting for available slot timed out"
            );
        }
        paddler_types::inference_client::Message::Response(_) => {
            panic!("expected an error response, got a successful response");
        }
    }
}

#[tokio::test]
#[file_serial]
async fn test_inference_fails_when_no_agents_registered() {
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
        1,
        Duration::from_millis(50),
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

    let result = balancer
        .client()
        .inference()
        .continue_from_raw_prompt(ContinueFromRawPromptParams {
            max_tokens: 10,
            raw_prompt: "Hello".to_string(),
        })
        .await;

    assert!(result.is_ok(), "WebSocket connection should succeed");

    let mut stream = result.unwrap();
    let first_message = stream.next().await;

    assert!(first_message.is_some(), "should receive a response message");

    let message = first_message.unwrap().expect("message should deserialize");

    match message {
        paddler_types::inference_client::Message::Error(envelope) => {
            assert_eq!(envelope.error.code, 504);
            assert_eq!(
                envelope.error.description,
                "Waiting for available slot timed out"
            );
        }
        paddler_types::inference_client::Message::Response(_) => {
            panic!("expected an error response, got a successful response");
        }
    }
}

#[tokio::test]
#[file_serial]
async fn test_balancer_overflows_buffer_when_feature_is_disabled() {
    let state_db = NamedTempFile::new().expect("failed to create temp file");

    let balancer = ManagedBalancer::spawn(balancer_params(
        BALANCER_MANAGEMENT_ADDR,
        BALANCER_INFERENCE_ADDR,
        state_db.path().to_str().unwrap(),
        0,
        Duration::from_millis(50),
    ))
    .await
    .expect("failed to spawn balancer");

    let _agent = ManagedAgent::spawn(ManagedAgentParams {
        management_addr: BALANCER_MANAGEMENT_ADDR.to_string(),
        name: Some("test-agent".to_string()),
        slots: 2,
    })
    .await
    .expect("failed to spawn agent");

    balancer.wait_for_agent_count(1).await;

    let result = balancer
        .client()
        .inference()
        .continue_from_raw_prompt(ContinueFromRawPromptParams {
            max_tokens: 10,
            raw_prompt: "Hello".to_string(),
        })
        .await;

    assert!(result.is_ok(), "WebSocket connection should succeed");

    let mut stream = result.unwrap();
    let first_message = stream.next().await;

    assert!(first_message.is_some(), "should receive a response message");

    let message = first_message.unwrap().expect("message should deserialize");

    match message {
        paddler_types::inference_client::Message::Error(envelope) => {
            assert_eq!(envelope.error.code, 503);
            assert_eq!(envelope.error.description, "Buffered requests overflow");
        }
        paddler_types::inference_client::Message::Response(_) => {
            panic!("expected buffer overflow error, got a successful response");
        }
    }
}

#[tokio::test]
#[file_serial]
async fn test_balancer_can_buffer_requests() {
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
        1,
        Duration::from_secs(120),
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

    let mut stream = balancer
        .client()
        .inference()
        .continue_from_raw_prompt(ContinueFromRawPromptParams {
            max_tokens: 10,
            raw_prompt: "Hello".to_string(),
        })
        .await
        .expect("WebSocket connection should succeed");

    balancer.wait_for_buffered_requests(1).await;

    let _agent = ManagedAgent::spawn(ManagedAgentParams {
        management_addr: BALANCER_MANAGEMENT_ADDR.to_string(),
        name: Some("buffered-agent".to_string()),
        slots: 4,
    })
    .await
    .expect("failed to spawn agent");

    let first_message = stream.next().await;

    assert!(first_message.is_some(), "should receive a response message");

    let message = first_message.unwrap().expect("message should deserialize");

    match message {
        paddler_types::inference_client::Message::Error(envelope) => {
            panic!(
                "expected a successful response, got error: {} - {}",
                envelope.error.code, envelope.error.description
            );
        }
        paddler_types::inference_client::Message::Response(_) => {}
    }
}
