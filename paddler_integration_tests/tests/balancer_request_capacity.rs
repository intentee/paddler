#![cfg(feature = "paddler_integration_tests")]

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
async fn test_slots_can_handle_request() {
    let state_db = NamedTempFile::new().expect("failed to create temp file");

    let balancer = ManagedBalancer::spawn(balancer_params(
        BALANCER_MANAGEMENT_ADDR,
        BALANCER_INFERENCE_ADDR,
        state_db.path().to_str().unwrap(),
        10,
        Duration::from_millis(50),
    ))
    .await
    .expect("failed to spawn balancer");

    let desired_state = BalancerDesiredState {
        chat_template_override: None,
        inference_parameters: InferenceParameters::default(),
        model: AGENT_DESIRED_MODEL.clone(),
        multimodal_projection: AgentDesiredModel::None,
        use_chat_template_override: false,
    };

    balancer
        .client()
        .management()
        .put_balancer_desired_state(&desired_state)
        .await
        .expect("failed to set balancer desired state");

    balancer.wait_for_desired_state(&desired_state).await;

    balancer.wait_for_agent_count(0).await;

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
            panic!("expected buffer overflow error, got a successful response");
        }
    }

    let _agent = ManagedAgent::spawn(ManagedAgentParams {
        management_addr: BALANCER_MANAGEMENT_ADDR.to_string(),
        name: Some("capacity-agent".to_string()),
        slots: 4,
    })
    .await
    .expect("failed to spawn agent");

    balancer.wait_for_agent_count(1).await;
    balancer.wait_for_total_slots(4).await;

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
            assert_ne!(
                envelope.error.code, 503,
                "request should not get buffer overflow after agent has slots"
            );
        }
        paddler_types::inference_client::Message::Response(_) => {}
    }
}
