#![cfg(all(
    feature = "tests_that_use_compiled_paddler",
    feature = "tests_that_use_llms"
))]

use std::time::Duration;

use anyhow::Context as _;
use anyhow::Result;
use anyhow::anyhow;
use futures_util::StreamExt;
use paddler_integration_tests::AGENT_DESIRED_MODEL;
use paddler_integration_tests::managed_agent::ManagedAgent;
use paddler_integration_tests::managed_agent_params::ManagedAgentParams;
use paddler_integration_tests::managed_balancer::ManagedBalancer;
use paddler_integration_tests::managed_balancer_params::ManagedBalancerParams;
use paddler_integration_tests::pick_balancer_addresses::pick_balancer_addresses;
use paddler_types::agent_desired_model::AgentDesiredModel;
use paddler_types::balancer_desired_state::BalancerDesiredState;
use paddler_types::inference_parameters::InferenceParameters;
use paddler_types::request_params::ContinueFromRawPromptParams;
use serial_test::file_serial;
use tempfile::NamedTempFile;

#[tokio::test]
#[file_serial]
async fn test_slots_can_handle_request() -> Result<()> {
    let state_db = NamedTempFile::new().context("failed to create temp file")?;
    let state_db_url = format!(
        "file://{}",
        state_db
            .path()
            .to_str()
            .context("temp file path is not valid UTF-8")?
    );
    let addresses = pick_balancer_addresses().context("pick addresses")?;

    let balancer = ManagedBalancer::spawn(ManagedBalancerParams {
        buffered_request_timeout: Duration::from_millis(50),
        compat_openai_addr: addresses.compat_openai,
        inference_addr: addresses.inference,
        inference_cors_allowed_hosts: vec![],
        inference_item_timeout: None,
        management_addr: addresses.management.clone(),
        management_cors_allowed_hosts: vec![],
        max_buffered_requests: 10,
        state_database_url: state_db_url,
    })
    .await
    .context("failed to spawn balancer")?;

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
        .context("failed to set balancer desired state")?;

    balancer.wait_for_desired_state(&desired_state).await;

    balancer.wait_for_agent_count(0).await;

    let result = balancer
        .client()
        .inference()
        .continue_from_raw_prompt(ContinueFromRawPromptParams {
            grammar: None,
            max_tokens: 10,
            raw_prompt: "Hello".to_owned(),
        })
        .await;

    let mut stream = result.context("WebSocket connection should succeed")?;

    let message = stream
        .next()
        .await
        .context("should receive a response message")?
        .context("message should deserialize")?;

    match message {
        paddler_types::inference_client::Message::Error(envelope) => {
            assert_eq!(envelope.error.code, 504);
            assert_eq!(
                envelope.error.description,
                "Waiting for available slot timed out"
            );
        }
        paddler_types::inference_client::Message::Response(_) => {
            return Err(anyhow!(
                "expected buffer overflow error, got a successful response"
            ));
        }
    }

    let _agent = ManagedAgent::spawn(&ManagedAgentParams {
        management_addr: addresses.management,
        name: Some("capacity-agent".to_owned()),
        slots: 4,
    })
    .context("failed to spawn agent")?;

    balancer.wait_for_agent_count(1).await;
    balancer.wait_for_total_slots(4).await;

    let result = balancer
        .client()
        .inference()
        .continue_from_raw_prompt(ContinueFromRawPromptParams {
            grammar: None,
            max_tokens: 10,
            raw_prompt: "Hello".to_owned(),
        })
        .await;

    let mut stream = result.context("WebSocket connection should succeed")?;

    let message = stream
        .next()
        .await
        .context("should receive a response message")?
        .context("message should deserialize")?;

    match message {
        paddler_types::inference_client::Message::Error(envelope) => {
            assert_ne!(
                envelope.error.code, 503,
                "request should not get buffer overflow after agent has slots"
            );
        }
        paddler_types::inference_client::Message::Response(_) => {}
    }

    Ok(())
}
