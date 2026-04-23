#![cfg(all(
    feature = "tests_that_use_compiled_paddler",
    feature = "tests_that_use_llms"
))]

use anyhow::Context as _;
use anyhow::Result;
use anyhow::anyhow;
use futures_util::StreamExt;
use paddler_integration_tests::AGENT_DESIRED_MODEL;
use paddler_integration_tests::managed_balancer::ManagedBalancer;
use paddler_integration_tests::managed_cluster::ManagedCluster;
use paddler_integration_tests::managed_cluster_params::ManagedClusterParams;
use paddler_types::agent_desired_model::AgentDesiredModel;
use paddler_types::balancer_desired_state::BalancerDesiredState;
use paddler_types::chat_template::ChatTemplate;
use paddler_types::conversation_history::ConversationHistory;
use paddler_types::conversation_message::ConversationMessage;
use paddler_types::conversation_message_content::ConversationMessageContent;
use paddler_types::generated_token_result::GeneratedTokenResult;
use paddler_types::huggingface_model_reference::HuggingFaceModelReference;
use paddler_types::inference_client::Message;
use paddler_types::inference_client::Response;
use paddler_types::inference_parameters::InferenceParameters;
use paddler_types::request_params::ContinueFromConversationHistoryParams;
use paddler_types::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;
use serial_test::file_serial;

const SIMPLE_CHAT_TEMPLATE: &str = "{{ messages[0].content }}";
const PREFIXED_CHAT_TEMPLATE: &str = "PREFIX:{{ messages[0].content }}";

fn chat_template_balancer_state(
    model: AgentDesiredModel,
    chat_template: ChatTemplate,
) -> BalancerDesiredState {
    BalancerDesiredState {
        chat_template_override: Some(chat_template),
        inference_parameters: InferenceParameters::default(),
        model,
        multimodal_projection: AgentDesiredModel::None,
        use_chat_template_override: true,
    }
}

fn chat_template_cluster_params(
    model: AgentDesiredModel,
    chat_template: ChatTemplate,
) -> ManagedClusterParams {
    ManagedClusterParams {
        agent_name: "chat-template-agent".to_owned(),
        agent_slots: 1,
        desired_state: chat_template_balancer_state(model, chat_template),
        ..ManagedClusterParams::default()
    }
}

async fn wait_for_chat_template_override(
    balancer: &ManagedBalancer,
    agent_id: &str,
    expected: &ChatTemplate,
) -> Result<()> {
    const CHAT_TEMPLATE_OVERRIDE_POLL_INTERVAL: std::time::Duration =
        std::time::Duration::from_millis(50);
    const CHAT_TEMPLATE_OVERRIDE_TIMEOUT: std::time::Duration =
        std::time::Duration::from_secs(30);

    let start = std::time::Instant::now();

    loop {
        let current = balancer
            .client()
            .management()
            .get_chat_template_override(agent_id)
            .await
            .context("failed to get chat template override")?;

        if current.as_ref() == Some(expected) {
            return Ok(());
        }

        if start.elapsed() > CHAT_TEMPLATE_OVERRIDE_TIMEOUT {
            return Err(anyhow!(
                "timed out waiting for agent to apply chat template override; current: {current:?}"
            ));
        }

        tokio::time::sleep(CHAT_TEMPLATE_OVERRIDE_POLL_INTERVAL).await;
    }
}

async fn swap_chat_template(
    balancer: &ManagedBalancer,
    agent_id: &str,
    model: AgentDesiredModel,
    new_template: ChatTemplate,
) -> Result<()> {
    let new_state = chat_template_balancer_state(model, new_template.clone());

    balancer
        .client()
        .management()
        .put_balancer_desired_state(&new_state)
        .await
        .context("failed to put new desired state")?;

    balancer.wait_for_desired_state(&new_state).await?;
    balancer
        .wait_for_applicable_state(&new_state.to_agent_desired_state())
        .await?;
    wait_for_chat_template_override(balancer, agent_id, &new_template).await?;

    Ok(())
}

async fn get_first_agent_id(balancer: &ManagedBalancer) -> Result<String> {
    let snapshot = balancer
        .client()
        .management()
        .get_agents()
        .await
        .context("failed to get agents")?;

    Ok(snapshot
        .agents
        .first()
        .context("should have at least one agent")?
        .id
        .clone())
}

async fn assert_agent_uses_chat_template_override(balancer: &ManagedBalancer) -> Result<()> {
    let snapshot = balancer
        .client()
        .management()
        .get_agents()
        .await
        .context("failed to get agents")?;

    let agent = snapshot
        .agents
        .first()
        .context("should have at least one agent")?;

    assert!(
        agent.uses_chat_template_override,
        "agent should use chat template override"
    );

    Ok(())
}

async fn assert_chat_template_renders_for_inference(balancer: &ManagedBalancer) -> Result<()> {
    let mut stream = balancer
        .client()
        .inference()
        .continue_from_conversation_history(ContinueFromConversationHistoryParams::<
            ValidatedParametersSchema,
        > {
            add_generation_prompt: true,
            conversation_history: ConversationHistory::new(vec![ConversationMessage {
                content: ConversationMessageContent::Text("The capital of France is".to_owned()),
                role: "user".to_owned(),
            }]),
            enable_thinking: false,
            grammar: None,
            max_tokens: 10,
            tools: vec![],
        })
        .await
        .context("conversation history request should succeed")?;

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
                    GeneratedTokenResult::ChatTemplateError(error) => {
                        return Err(anyhow!("chat template error: {error}"));
                    }
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
        "should have received tokens proving the chat template rendered the prompt"
    );

    Ok(())
}

#[tokio::test]
#[file_serial]
async fn test_agent_can_use_chat_template_for_model() -> Result<()> {
    let chat_template = ChatTemplate {
        content: SIMPLE_CHAT_TEMPLATE.to_owned(),
    };

    let cluster = ManagedCluster::spawn(chat_template_cluster_params(
        AgentDesiredModel::HuggingFace(HuggingFaceModelReference {
            filename: "nomic-embed-text-v1.5.Q2_K.gguf".to_owned(),
            repo_id: "nomic-ai/nomic-embed-text-v1.5-GGUF".to_owned(),
            revision: "main".to_owned(),
        }),
        chat_template.clone(),
    ))
    .await
    .context("failed to spawn cluster")?;

    assert_agent_uses_chat_template_override(&cluster.balancer).await?;

    let agent_id = get_first_agent_id(&cluster.balancer).await?;

    let retrieved_template = cluster
        .balancer
        .client()
        .management()
        .get_chat_template_override(&agent_id)
        .await
        .context("failed to get chat template override")?;

    assert_eq!(
        retrieved_template,
        Some(chat_template),
        "agent should have the provided chat template override"
    );

    assert_chat_template_renders_for_inference(&cluster.balancer).await?;

    Ok(())
}

#[tokio::test]
#[file_serial]
async fn test_agent_overrides_chat_template() -> Result<()> {
    let chat_template = ChatTemplate {
        content: SIMPLE_CHAT_TEMPLATE.to_owned(),
    };

    let cluster = ManagedCluster::spawn(chat_template_cluster_params(
        AGENT_DESIRED_MODEL.clone(),
        chat_template.clone(),
    ))
    .await
    .context("failed to spawn cluster")?;

    assert_agent_uses_chat_template_override(&cluster.balancer).await?;

    let agent_id = get_first_agent_id(&cluster.balancer).await?;

    let retrieved_template = cluster
        .balancer
        .client()
        .management()
        .get_chat_template_override(&agent_id)
        .await
        .context("failed to get chat template override")?;

    assert_eq!(
        retrieved_template,
        Some(chat_template),
        "agent should have the override template instead of the built-in one"
    );

    assert_chat_template_renders_for_inference(&cluster.balancer).await?;

    Ok(())
}

#[tokio::test]
#[file_serial]
async fn test_agent_swaps_chat_template_between_inference_calls() -> Result<()> {
    let template_a = ChatTemplate {
        content: SIMPLE_CHAT_TEMPLATE.to_owned(),
    };
    let template_b = ChatTemplate {
        content: PREFIXED_CHAT_TEMPLATE.to_owned(),
    };

    let cluster = ManagedCluster::spawn(chat_template_cluster_params(
        AGENT_DESIRED_MODEL.clone(),
        template_a.clone(),
    ))
    .await
    .context("failed to spawn cluster")?;

    let agent_id = get_first_agent_id(&cluster.balancer).await?;

    assert_chat_template_renders_for_inference(&cluster.balancer).await?;

    swap_chat_template(
        &cluster.balancer,
        &agent_id,
        AGENT_DESIRED_MODEL.clone(),
        template_b.clone(),
    )
    .await?;

    let retrieved_template = cluster
        .balancer
        .client()
        .management()
        .get_chat_template_override(&agent_id)
        .await
        .context("failed to get chat template override")?;

    assert_eq!(
        retrieved_template,
        Some(template_b),
        "agent should have swapped to the new chat template override"
    );

    assert_chat_template_renders_for_inference(&cluster.balancer).await?;

    Ok(())
}

#[tokio::test]
#[file_serial]
async fn test_agent_drains_in_flight_then_applies_chat_template_swap() -> Result<()> {
    let template_a = ChatTemplate {
        content: SIMPLE_CHAT_TEMPLATE.to_owned(),
    };
    let template_b = ChatTemplate {
        content: PREFIXED_CHAT_TEMPLATE.to_owned(),
    };

    let cluster = ManagedCluster::spawn(chat_template_cluster_params(
        AGENT_DESIRED_MODEL.clone(),
        template_a.clone(),
    ))
    .await
    .context("failed to spawn cluster")?;

    let agent_id = get_first_agent_id(&cluster.balancer).await?;

    let mut stream = cluster
        .balancer
        .client()
        .inference()
        .continue_from_conversation_history(ContinueFromConversationHistoryParams::<
            ValidatedParametersSchema,
        > {
            add_generation_prompt: true,
            conversation_history: ConversationHistory::new(vec![ConversationMessage {
                content: ConversationMessageContent::Text("The capital of France is".to_owned()),
                role: "user".to_owned(),
            }]),
            enable_thinking: false,
            grammar: None,
            max_tokens: 10,
            tools: vec![],
        })
        .await
        .context("conversation history request should succeed")?;

    let first_message = stream
        .next()
        .await
        .ok_or_else(|| anyhow!("stream ended before the first message"))?
        .context("first message should deserialize")?;

    match first_message {
        Message::Response(envelope) => match envelope.response {
            Response::GeneratedToken(GeneratedTokenResult::Token(_)) => {}
            Response::GeneratedToken(GeneratedTokenResult::Done) => {
                return Err(anyhow!(
                    "in-flight request completed before producing any tokens"
                ));
            }
            Response::GeneratedToken(GeneratedTokenResult::ChatTemplateError(error)) => {
                return Err(anyhow!("chat template error: {error}"));
            }
            Response::GeneratedToken(other) => {
                return Err(anyhow!("unexpected token result: {other:?}"));
            }
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

    let new_state = chat_template_balancer_state(AGENT_DESIRED_MODEL.clone(), template_b.clone());

    cluster
        .balancer
        .client()
        .management()
        .put_balancer_desired_state(&new_state)
        .await
        .context("failed to put new desired state")?;

    let mut received_done = false;

    while let Some(message) = stream.next().await {
        let message = message.context("message should deserialize")?;

        match message {
            Message::Response(envelope) => match envelope.response {
                Response::GeneratedToken(token_result) => match token_result {
                    GeneratedTokenResult::Token(_) => {}
                    GeneratedTokenResult::Done => {
                        received_done = true;

                        break;
                    }
                    GeneratedTokenResult::ChatTemplateError(error) => {
                        return Err(anyhow!(
                            "in-flight stream must not see ChatTemplateError during swap: {error}"
                        ));
                    }
                    other => return Err(anyhow!("unexpected token result: {other:?}")),
                },
                other => return Err(anyhow!("unexpected response: {other:?}")),
            },
            Message::Error(envelope) => {
                return Err(anyhow!(
                    "in-flight stream must not error during swap: {} - {}",
                    envelope.error.code,
                    envelope.error.description
                ));
            }
        }
    }

    assert!(
        received_done,
        "in-flight stream must terminate cleanly with Done despite the concurrent swap"
    );

    cluster.balancer.wait_for_desired_state(&new_state).await?;
    cluster
        .balancer
        .wait_for_applicable_state(&new_state.to_agent_desired_state())
        .await?;
    wait_for_chat_template_override(&cluster.balancer, &agent_id, &template_b).await?;

    let retrieved_template = cluster
        .balancer
        .client()
        .management()
        .get_chat_template_override(&agent_id)
        .await
        .context("failed to get chat template override")?;

    assert_eq!(
        retrieved_template,
        Some(template_b),
        "agent should have applied the new chat template override after draining"
    );

    assert_chat_template_renders_for_inference(&cluster.balancer).await?;

    Ok(())
}
