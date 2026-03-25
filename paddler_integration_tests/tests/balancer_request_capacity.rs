#![cfg(all(feature = "tests_that_use_compiled_paddler", feature = "tests_that_use_llms"))]

use std::time::Duration;

use futures_util::StreamExt;
use paddler_integration_tests::AGENT_DESIRED_MODEL;
use paddler_integration_tests::BALANCER_INFERENCE_ADDR;
use paddler_integration_tests::BALANCER_MANAGEMENT_ADDR;
use paddler_integration_tests::BALANCER_OPENAI_ADDR;
use paddler_integration_tests::managed_agent::ManagedAgent;
use paddler_integration_tests::managed_agent::ManagedAgentParams;
use paddler_integration_tests::managed_balancer::ManagedBalancer;
use paddler_integration_tests::managed_balancer::ManagedBalancerParams;
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
    let state_db_url = format!("file://{}", state_db.path().to_str().unwrap());

    let balancer = ManagedBalancer::spawn(ManagedBalancerParams {
        buffered_request_timeout: Duration::from_millis(50),
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

    let _agent = ManagedAgent::spawn(&ManagedAgentParams {
        management_addr: BALANCER_MANAGEMENT_ADDR.to_string(),
        name: Some("capacity-agent".to_string()),
        slots: 4,
    })
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
