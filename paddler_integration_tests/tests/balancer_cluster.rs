#![cfg(all(
    feature = "tests_that_use_compiled_paddler",
    feature = "tests_that_use_llms"
))]

use std::pin::Pin;
use std::time::Duration;

use anyhow::Context as _;
use anyhow::Result;
use anyhow::anyhow;
use futures_util::Stream;
use futures_util::StreamExt;
use paddler_integration_tests::AGENT_DESIRED_MODEL;
use paddler_integration_tests::balancer_addresses::BalancerAddresses;
use paddler_integration_tests::managed_agent::ManagedAgent;
use paddler_integration_tests::managed_agent_params::ManagedAgentParams;
use paddler_integration_tests::managed_balancer::ManagedBalancer;
use paddler_integration_tests::managed_balancer_params::ManagedBalancerParams;
use paddler_integration_tests::managed_cluster::ManagedCluster;
use paddler_integration_tests::managed_cluster_params::ManagedClusterParams;
use paddler_integration_tests::pick_balancer_addresses::pick_balancer_addresses;
use paddler_types::agent_desired_model::AgentDesiredModel;
use paddler_types::balancer_desired_state::BalancerDesiredState;
use paddler_types::generated_token_result::GeneratedTokenResult;
use paddler_types::grammar_constraint::GrammarConstraint;
use paddler_types::inference_client::Message;
use paddler_types::inference_client::Response;
use paddler_types::inference_parameters::InferenceParameters;
use paddler_types::request_params::ContinueFromRawPromptParams;
use serial_test::file_serial;
use tempfile::NamedTempFile;

type InferenceStream =
    Pin<Box<dyn Stream<Item = paddler_client::Result<Message>> + Send + 'static>>;

fn balancer_params(
    addresses: &BalancerAddresses,
    buffered_request_timeout: Duration,
    inference_item_timeout: Option<Duration>,
    max_buffered_requests: i32,
    state_database_url: String,
) -> ManagedBalancerParams {
    ManagedBalancerParams {
        buffered_request_timeout,
        compat_openai_addr: addresses.compat_openai.clone(),
        inference_addr: addresses.inference.clone(),
        inference_cors_allowed_hosts: vec![],
        inference_item_timeout,
        management_addr: addresses.management.clone(),
        management_cors_allowed_hosts: vec![],
        max_buffered_requests,
        state_database_url,
    }
}

async fn send_buffered_requests(
    balancer: &ManagedBalancer,
    count: usize,
) -> Result<Vec<InferenceStream>> {
    let mut streams = Vec::with_capacity(count);

    for _ in 0..count {
        let stream = balancer
            .client()
            .inference()
            .continue_from_raw_prompt(ContinueFromRawPromptParams {
                grammar: None,
                max_tokens: 10,
                raw_prompt: "Hello".to_owned(),
            })
            .await
            .context("WebSocket connection should succeed")?;

        streams.push(stream);
    }

    Ok(streams)
}

#[tokio::test]
#[file_serial]
async fn test_health_endpoint_returns_ok() -> Result<()> {
    let state_db = NamedTempFile::new().context("failed to create temp file")?;
    let state_db_url = format!(
        "file://{}",
        state_db
            .path()
            .to_str()
            .context("temp file path is not valid UTF-8")?
    );
    let addresses = pick_balancer_addresses().context("pick addresses")?;

    let balancer = ManagedBalancer::spawn(balancer_params(
        &addresses,
        Duration::from_secs(10),
        None,
        30,
        state_db_url,
    ))
    .await
    .context("failed to spawn balancer")?;

    let health = balancer
        .client()
        .management()
        .get_health()
        .await
        .context("health endpoint should respond")?;

    assert_eq!(health, "OK");

    Ok(())
}

#[tokio::test]
#[file_serial]
async fn test_inference_fails_when_no_model_configured() -> Result<()> {
    let state_db = NamedTempFile::new().context("failed to create temp file")?;
    let state_db_url = format!(
        "file://{}",
        state_db
            .path()
            .to_str()
            .context("temp file path is not valid UTF-8")?
    );
    let addresses = pick_balancer_addresses().context("pick addresses")?;

    let balancer = ManagedBalancer::spawn(balancer_params(
        &addresses,
        Duration::from_secs(10),
        None,
        30,
        state_db_url,
    ))
    .await
    .context("failed to spawn balancer")?;

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
                "expected an error response, got a successful response"
            ));
        }
    }

    Ok(())
}

#[tokio::test]
#[file_serial]
async fn test_inference_fails_when_no_agents_registered() -> Result<()> {
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
        model: AGENT_DESIRED_MODEL.clone(),
        multimodal_projection: AgentDesiredModel::None,
        use_chat_template_override: false,
    };

    let balancer = ManagedBalancer::spawn(balancer_params(
        &addresses,
        Duration::from_millis(50),
        None,
        1,
        state_db_url,
    ))
    .await
    .context("failed to spawn balancer")?;

    balancer
        .client()
        .management()
        .put_balancer_desired_state(&desired_state)
        .await
        .context("failed to set balancer desired state")?;

    balancer.wait_for_desired_state(&desired_state).await?;

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
                "expected an error response, got a successful response"
            ));
        }
    }

    Ok(())
}

#[tokio::test]
#[file_serial]
async fn test_balancer_overflows_buffer_when_feature_is_disabled() -> Result<()> {
    let state_db = NamedTempFile::new().context("failed to create temp file")?;
    let state_db_url = format!(
        "file://{}",
        state_db
            .path()
            .to_str()
            .context("temp file path is not valid UTF-8")?
    );
    let addresses = pick_balancer_addresses().context("pick addresses")?;

    let balancer = ManagedBalancer::spawn(balancer_params(
        &addresses,
        Duration::from_millis(50),
        None,
        0,
        state_db_url,
    ))
    .await
    .context("failed to spawn balancer")?;

    let _agent = ManagedAgent::spawn(&ManagedAgentParams {
        management_addr: addresses.management.clone(),
        name: Some("test-agent".to_owned()),
        slots: 2,
    })
    .context("failed to spawn agent")?;

    balancer.wait_for_agent_count(1).await?;

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
            assert_eq!(envelope.error.code, 503);
            assert_eq!(envelope.error.description, "Buffered requests overflow");
        }
        paddler_types::inference_client::Message::Response(_) => {
            return Err(anyhow!(
                "expected buffer overflow error, got a successful response"
            ));
        }
    }

    Ok(())
}

#[tokio::test]
#[file_serial]
async fn test_balancer_can_buffer_requests() -> Result<()> {
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
        model: AGENT_DESIRED_MODEL.clone(),
        multimodal_projection: AgentDesiredModel::None,
        use_chat_template_override: false,
    };

    let balancer = ManagedBalancer::spawn(balancer_params(
        &addresses,
        Duration::from_secs(120),
        None,
        1,
        state_db_url,
    ))
    .await
    .context("failed to spawn balancer")?;

    balancer
        .client()
        .management()
        .put_balancer_desired_state(&desired_state)
        .await
        .context("failed to set balancer desired state")?;

    balancer.wait_for_desired_state(&desired_state).await?;

    let mut stream = balancer
        .client()
        .inference()
        .continue_from_raw_prompt(ContinueFromRawPromptParams {
            grammar: None,
            max_tokens: 10,
            raw_prompt: "Hello".to_owned(),
        })
        .await
        .context("WebSocket connection should succeed")?;

    balancer.wait_for_buffered_requests(1).await?;

    let _agent = ManagedAgent::spawn(&ManagedAgentParams {
        management_addr: addresses.management.clone(),
        name: Some("buffered-agent".to_owned()),
        slots: 4,
    })
    .context("failed to spawn agent")?;

    let first_message = stream.next().await;

    let message = first_message
        .context("should receive a response message")?
        .context("message should deserialize")?;

    match message {
        paddler_types::inference_client::Message::Error(envelope) => {
            return Err(anyhow!(
                "expected a successful response, got error: {} - {}",
                envelope.error.code,
                envelope.error.description
            ));
        }
        paddler_types::inference_client::Message::Response(_) => {}
    }

    Ok(())
}

#[tokio::test]
#[file_serial]
async fn test_balancer_distributes_buffered_requests_across_multiple_agents() -> Result<()> {
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
        model: AGENT_DESIRED_MODEL.clone(),
        multimodal_projection: AgentDesiredModel::None,
        use_chat_template_override: false,
    };

    let balancer = ManagedBalancer::spawn(balancer_params(
        &addresses,
        Duration::from_secs(120),
        None,
        10,
        state_db_url,
    ))
    .await
    .context("failed to spawn balancer")?;

    balancer
        .client()
        .management()
        .put_balancer_desired_state(&desired_state)
        .await
        .context("failed to set balancer desired state")?;

    balancer.wait_for_desired_state(&desired_state).await?;

    let _agent_one = ManagedAgent::spawn(&ManagedAgentParams {
        management_addr: addresses.management.clone(),
        name: Some("distributed-agent-one".to_owned()),
        slots: 2,
    })
    .context("failed to spawn first agent")?;

    let _agent_two = ManagedAgent::spawn(&ManagedAgentParams {
        management_addr: addresses.management.clone(),
        name: Some("distributed-agent-two".to_owned()),
        slots: 2,
    })
    .context("failed to spawn second agent")?;

    balancer.wait_for_total_slots(4).await?;

    let mut streams = send_buffered_requests(&balancer, 5).await?;

    let mut successful_responses = 0;

    for stream in &mut streams {
        let first_message = stream.next().await;

        let message = first_message
            .context("should receive a response message")?
            .context("message should deserialize")?;

        match message {
            Message::Response(_) => {
                successful_responses += 1;
            }
            Message::Error(envelope) => {
                return Err(anyhow!(
                    "expected a successful response, got error: {} - {}",
                    envelope.error.code,
                    envelope.error.description
                ));
            }
        }
    }

    assert_eq!(
        successful_responses, 5,
        "all 5 requests should receive successful responses"
    );

    Ok(())
}

#[tokio::test]
#[file_serial]
async fn test_buffered_requests_when_agent_is_removed() -> Result<()> {
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
        model: AGENT_DESIRED_MODEL.clone(),
        multimodal_projection: AgentDesiredModel::None,
        use_chat_template_override: false,
    };

    let balancer = ManagedBalancer::spawn(balancer_params(
        &addresses,
        Duration::from_secs(120),
        None,
        10,
        state_db_url,
    ))
    .await
    .context("failed to spawn balancer")?;

    balancer
        .client()
        .management()
        .put_balancer_desired_state(&desired_state)
        .await
        .context("failed to set balancer desired state")?;

    balancer.wait_for_desired_state(&desired_state).await?;

    let mut streams = send_buffered_requests(&balancer, 3).await?;

    balancer.wait_for_buffered_requests(3).await?;

    let _agent_one = ManagedAgent::spawn(&ManagedAgentParams {
        management_addr: addresses.management.clone(),
        name: Some("removable-agent-one".to_owned()),
        slots: 2,
    })
    .context("failed to spawn first agent")?;

    let mut agent_two = ManagedAgent::spawn(&ManagedAgentParams {
        management_addr: addresses.management.clone(),
        name: Some("removable-agent-two".to_owned()),
        slots: 2,
    })
    .context("failed to spawn second agent")?;

    balancer.wait_for_agent_count(2).await?;

    agent_two.kill();

    balancer.wait_for_agent_count(1).await?;

    let mut successful_responses = 0;
    let mut error_responses = 0;

    for stream in &mut streams {
        let first_message = stream.next().await;

        let message = first_message
            .context("should receive a response message")?
            .context("message should deserialize")?;

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

    Ok(())
}

#[tokio::test]
#[file_serial]
async fn test_inference_item_timeout_zero_causes_immediate_timeout() -> Result<()> {
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
        model: AGENT_DESIRED_MODEL.clone(),
        multimodal_projection: AgentDesiredModel::None,
        use_chat_template_override: false,
    };

    let balancer = ManagedBalancer::spawn(balancer_params(
        &addresses,
        Duration::from_secs(10),
        Some(Duration::ZERO),
        10,
        state_db_url,
    ))
    .await
    .context("failed to spawn balancer")?;

    balancer
        .client()
        .management()
        .put_balancer_desired_state(&desired_state)
        .await
        .context("failed to set balancer desired state")?;

    balancer.wait_for_desired_state(&desired_state).await?;

    let _agent = ManagedAgent::spawn(&ManagedAgentParams {
        management_addr: addresses.management.clone(),
        name: Some("timeout-agent".to_owned()),
        slots: 1,
    })
    .context("failed to spawn agent")?;

    balancer.wait_for_agent_count(1).await?;
    balancer.wait_for_total_slots(1).await?;

    let mut stream = balancer
        .client()
        .inference()
        .continue_from_raw_prompt(ContinueFromRawPromptParams {
            grammar: None,
            max_tokens: 10,
            raw_prompt: "Hello".to_owned(),
        })
        .await
        .context("WebSocket connection should succeed")?;

    let first_message = stream.next().await;

    let message = first_message
        .context("should receive a response message")?
        .context("message should deserialize")?;

    match message {
        Message::Error(envelope) => {
            assert_eq!(envelope.error.code, 504);
            assert_eq!(
                envelope.error.description,
                "Inference timed out after 0ms waiting for next token. Increase --inference-item-timeout if the prompt requires longer processing."
            );
        }
        Message::Response(_) => {
            return Err(anyhow!("expected timeout error, got a successful response"));
        }
    }

    Ok(())
}

#[tokio::test]
#[file_serial]
async fn test_buffered_requests_stream_receives_snapshot() -> Result<()> {
    let state_db = NamedTempFile::new().context("failed to create temp file")?;
    let state_db_url = format!(
        "file://{}",
        state_db
            .path()
            .to_str()
            .context("temp file path is not valid UTF-8")?
    );
    let addresses = pick_balancer_addresses().context("pick addresses")?;

    let balancer = ManagedBalancer::spawn(balancer_params(
        &addresses,
        Duration::from_secs(10),
        None,
        10,
        state_db_url,
    ))
    .await
    .context("failed to spawn balancer")?;

    let mut stream = balancer
        .client()
        .management()
        .buffered_requests_stream()
        .await
        .context("buffered requests stream should connect")?;

    let first_event = stream
        .next()
        .await
        .context("stream must produce at least one event")?
        .context("first event should deserialize")?;

    assert!(
        first_event.buffered_requests_current >= 0,
        "buffered request count must be non-negative"
    );

    Ok(())
}

#[tokio::test]
#[file_serial]
async fn test_in_flight_requests_drain_before_model_switch() -> Result<()> {
    let expected_output = "the quick brown fox jumps over the lazy dog";

    let cluster = ManagedCluster::spawn(ManagedClusterParams {
        agent_slots: 1,
        ..ManagedClusterParams::default()
    })
    .await
    .context("failed to spawn cluster")?;

    let mut stream = cluster
        .balancer
        .client()
        .inference()
        .continue_from_raw_prompt(ContinueFromRawPromptParams {
            grammar: Some(GrammarConstraint::Gbnf {
                grammar: format!("root ::= \"{expected_output}\""),
                root: "root".to_owned(),
            }),
            max_tokens: 200,
            raw_prompt: "Say the following: the quick brown fox jumps over the lazy dog".to_owned(),
        })
        .await
        .context("inference request should connect")?;

    let switch_state = BalancerDesiredState {
        chat_template_override: None,
        inference_parameters: InferenceParameters::default(),
        model: AgentDesiredModel::LocalToAgent("/nonexistent/model.gguf".to_owned()),
        multimodal_projection: AgentDesiredModel::None,
        use_chat_template_override: false,
    };

    cluster
        .balancer
        .client()
        .management()
        .put_balancer_desired_state(&switch_state)
        .await
        .context("failed to trigger model switch")?;

    let mut text = String::new();

    while let Some(message) = stream.next().await {
        let message = message.context("message should deserialize")?;

        match message {
            Message::Response(envelope) => match envelope.response {
                Response::GeneratedToken(GeneratedTokenResult::Token(token)) => {
                    text.push_str(&token);
                }
                Response::GeneratedToken(GeneratedTokenResult::Done) => break,
                other => return Err(anyhow!("unexpected response during drain test: {other:?}")),
            },
            Message::Error(envelope) => {
                return Err(anyhow!(
                    "request failed during model switch (drain did not protect in-flight request): {} - {}",
                    envelope.error.code,
                    envelope.error.description
                ));
            }
        }
    }

    assert_eq!(
        text, expected_output,
        "grammar-constrained output should complete fully despite concurrent model switch"
    );

    Ok(())
}
