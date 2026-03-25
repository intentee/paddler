#![cfg(feature = "tests_that_use_compiled_paddler")]

use std::pin::Pin;
use std::time::Duration;

use futures_util::Stream;
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
use paddler_types::inference_client::Message;
use paddler_types::inference_parameters::InferenceParameters;
use paddler_types::request_params::ContinueFromRawPromptParams;
use serial_test::file_serial;
use tempfile::NamedTempFile;

type InferenceStream =
    Pin<Box<dyn Stream<Item = paddler_client::Result<Message>> + Send + 'static>>;

async fn send_buffered_requests(balancer: &ManagedBalancer, count: usize) -> Vec<InferenceStream> {
    let mut streams = Vec::with_capacity(count);

    for _ in 0..count {
        let stream = balancer
            .client()
            .inference()
            .continue_from_raw_prompt(ContinueFromRawPromptParams {
                max_tokens: 10,
                raw_prompt: "Hello".to_string(),
            })
            .await
            .expect("WebSocket connection should succeed");

        streams.push(stream);
    }

    streams
}

#[tokio::test]
#[file_serial]
async fn test_health_endpoint_returns_ok() {
    let state_db = NamedTempFile::new().expect("failed to create temp file");
    let state_db_url = format!("file://{}", state_db.path().to_str().unwrap());

    let balancer = ManagedBalancer::spawn(balancer_params(
        BALANCER_MANAGEMENT_ADDR,
        BALANCER_INFERENCE_ADDR,
        &state_db_url,
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
    let state_db_url = format!("file://{}", state_db.path().to_str().unwrap());

    let balancer = ManagedBalancer::spawn(balancer_params(
        BALANCER_MANAGEMENT_ADDR,
        BALANCER_INFERENCE_ADDR,
        &state_db_url,
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
    let state_db_url = format!("file://{}", state_db.path().to_str().unwrap());

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
        &state_db_url,
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
    let state_db_url = format!("file://{}", state_db.path().to_str().unwrap());

    let balancer = ManagedBalancer::spawn(balancer_params(
        BALANCER_MANAGEMENT_ADDR,
        BALANCER_INFERENCE_ADDR,
        &state_db_url,
        0,
        Duration::from_millis(50),
    ))
    .await
    .expect("failed to spawn balancer");

    let _agent = ManagedAgent::spawn(&ManagedAgentParams {
        management_addr: BALANCER_MANAGEMENT_ADDR.to_string(),
        name: Some("test-agent".to_string()),
        slots: 2,
    })
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
    let state_db_url = format!("file://{}", state_db.path().to_str().unwrap());

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
        &state_db_url,
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

    let _agent = ManagedAgent::spawn(&ManagedAgentParams {
        management_addr: BALANCER_MANAGEMENT_ADDR.to_string(),
        name: Some("buffered-agent".to_string()),
        slots: 4,
    })
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

#[tokio::test]
#[file_serial]
async fn test_balancer_distributes_buffered_requests_across_multiple_agents() {
    let state_db = NamedTempFile::new().expect("failed to create temp file");
    let state_db_url = format!("file://{}", state_db.path().to_str().unwrap());

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
        &state_db_url,
        10,
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

    let mut streams = send_buffered_requests(&balancer, 7).await;

    balancer.wait_for_buffered_requests(7).await;

    let _agent_one = ManagedAgent::spawn(&ManagedAgentParams {
        management_addr: BALANCER_MANAGEMENT_ADDR.to_string(),
        name: Some("distributed-agent-one".to_string()),
        slots: 3,
    })
    .expect("failed to spawn first agent");

    let _agent_two = ManagedAgent::spawn(&ManagedAgentParams {
        management_addr: BALANCER_MANAGEMENT_ADDR.to_string(),
        name: Some("distributed-agent-two".to_string()),
        slots: 3,
    })
    .expect("failed to spawn second agent");

    let mut successful_responses = 0;

    for stream in &mut streams {
        let first_message = stream.next().await;

        assert!(first_message.is_some(), "should receive a response message");

        let message = first_message.unwrap().expect("message should deserialize");

        match message {
            Message::Response(_) => {
                successful_responses += 1;
            }
            Message::Error(envelope) => {
                panic!(
                    "expected a successful response, got error: {} - {}",
                    envelope.error.code, envelope.error.description
                );
            }
        }
    }

    assert_eq!(
        successful_responses, 7,
        "all 7 buffered requests should receive successful responses"
    );
}

#[tokio::test]
#[file_serial]
async fn test_buffered_requests_when_agent_is_removed() {
    let state_db = NamedTempFile::new().expect("failed to create temp file");
    let state_db_url = format!("file://{}", state_db.path().to_str().unwrap());

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
        &state_db_url,
        10,
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

    let mut streams = send_buffered_requests(&balancer, 3).await;

    balancer.wait_for_buffered_requests(3).await;

    let _agent_one = ManagedAgent::spawn(&ManagedAgentParams {
        management_addr: BALANCER_MANAGEMENT_ADDR.to_string(),
        name: Some("removable-agent-one".to_string()),
        slots: 2,
    })
    .expect("failed to spawn first agent");

    let mut agent_two = ManagedAgent::spawn(&ManagedAgentParams {
        management_addr: BALANCER_MANAGEMENT_ADDR.to_string(),
        name: Some("removable-agent-two".to_string()),
        slots: 2,
    })
    .expect("failed to spawn second agent");

    balancer.wait_for_agent_count(2).await;

    agent_two.kill();

    balancer.wait_for_agent_count(1).await;

    let mut successful_responses = 0;
    let mut error_responses = 0;

    for stream in &mut streams {
        let first_message = stream.next().await;

        assert!(first_message.is_some(), "should receive a response message");

        let message = first_message.unwrap().expect("message should deserialize");

        match message {
            Message::Response(_) => {
                successful_responses += 1;
            }
            Message::Error(_) => {
                error_responses += 1;
            }
        }
    }

    assert!(
        successful_responses > 0,
        "at least some requests should succeed via the remaining agent"
    );

    assert_eq!(
        successful_responses + error_responses,
        3,
        "all 3 requests should have resolved"
    );
}

#[tokio::test]
#[file_serial]
async fn test_inference_item_timeout_zero_causes_immediate_timeout() {
    let state_db = NamedTempFile::new().expect("failed to create temp file");
    let state_db_url = format!("file://{}", state_db.path().to_str().unwrap());

    let desired_state = BalancerDesiredState {
        chat_template_override: None,
        inference_parameters: InferenceParameters::default(),
        model: AGENT_DESIRED_MODEL.clone(),
        multimodal_projection: AgentDesiredModel::None,
        use_chat_template_override: false,
    };

    let mut params = balancer_params(
        BALANCER_MANAGEMENT_ADDR,
        BALANCER_INFERENCE_ADDR,
        &state_db_url,
        10,
        Duration::from_secs(10),
    );

    params.inference_item_timeout = Some(Duration::ZERO);

    let balancer = ManagedBalancer::spawn(params)
        .await
        .expect("failed to spawn balancer");

    balancer
        .client()
        .management()
        .put_balancer_desired_state(&desired_state)
        .await
        .expect("failed to set balancer desired state");

    balancer.wait_for_desired_state(&desired_state).await;

    let _agent = ManagedAgent::spawn(&ManagedAgentParams {
        management_addr: BALANCER_MANAGEMENT_ADDR.to_string(),
        name: Some("timeout-agent".to_string()),
        slots: 1,
    })
    .expect("failed to spawn agent");

    balancer.wait_for_agent_count(1).await;
    balancer.wait_for_total_slots(1).await;

    let mut stream = balancer
        .client()
        .inference()
        .continue_from_raw_prompt(ContinueFromRawPromptParams {
            max_tokens: 10,
            raw_prompt: "Hello".to_string(),
        })
        .await
        .expect("WebSocket connection should succeed");

    let first_message = stream.next().await;

    assert!(first_message.is_some(), "should receive a response message");

    let message = first_message.unwrap().expect("message should deserialize");

    match message {
        Message::Error(envelope) => {
            assert_eq!(envelope.error.code, 504);
            assert_eq!(
                envelope.error.description,
                "Inference timed out after 0ms waiting for next token. Increase --inference-item-timeout if the prompt requires longer processing."
            );
        }
        Message::Response(_) => {
            panic!("expected timeout error, got a successful response");
        }
    }
}

#[tokio::test]
#[file_serial]
async fn test_buffered_requests_stream_receives_snapshot() {
    let state_db = NamedTempFile::new().expect("failed to create temp file");
    let state_db_url = format!("file://{}", state_db.path().to_str().unwrap());

    let balancer = ManagedBalancer::spawn(balancer_params(
        BALANCER_MANAGEMENT_ADDR,
        BALANCER_INFERENCE_ADDR,
        &state_db_url,
        10,
        Duration::from_secs(10),
    ))
    .await
    .expect("failed to spawn balancer");

    let mut stream = balancer
        .client()
        .management()
        .buffered_requests_stream()
        .await
        .expect("buffered requests stream should connect");

    let first_event = stream
        .next()
        .await
        .expect("stream must produce at least one event")
        .expect("first event should deserialize");

    assert!(
        first_event.buffered_requests_current >= 0,
        "buffered request count must be non-negative"
    );
}
