#![cfg(all(
    feature = "tests_that_use_compiled_paddler",
    feature = "tests_that_use_llms"
))]

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

fn chat_template_cluster_params(
    model: AgentDesiredModel,
    chat_template: ChatTemplate,
) -> ManagedClusterParams {
    ManagedClusterParams {
        agent_name: "chat-template-agent".to_string(),
        agent_slots: 1,
        desired_state: BalancerDesiredState {
            chat_template_override: Some(chat_template),
            inference_parameters: InferenceParameters::default(),
            model,
            multimodal_projection: AgentDesiredModel::None,
            use_chat_template_override: true,
        },
        ..ManagedClusterParams::default()
    }
}

async fn get_first_agent_id(balancer: &ManagedBalancer) -> String {
    let snapshot = balancer
        .client()
        .management()
        .get_agents()
        .await
        .expect("failed to get agents");

    snapshot
        .agents
        .first()
        .expect("should have at least one agent")
        .id
        .clone()
}

async fn assert_agent_uses_chat_template_override(balancer: &ManagedBalancer) {
    let snapshot = balancer
        .client()
        .management()
        .get_agents()
        .await
        .expect("failed to get agents");

    let agent = snapshot
        .agents
        .first()
        .expect("should have at least one agent");

    assert!(
        agent.uses_chat_template_override,
        "agent should use chat template override"
    );
}

async fn assert_chat_template_renders_for_inference(balancer: &ManagedBalancer) {
    let mut stream = balancer
        .client()
        .inference()
        .continue_from_conversation_history(ContinueFromConversationHistoryParams::<
            ValidatedParametersSchema,
        > {
            add_generation_prompt: true,
            conversation_history: ConversationHistory::new(vec![ConversationMessage {
                content: ConversationMessageContent::Text("The capital of France is".to_string()),
                role: "user".to_string(),
            }]),
            enable_thinking: false,
            grammar: None,
            max_tokens: 10,
            tools: vec![],
        })
        .await
        .expect("conversation history request should succeed");

    let mut received_tokens = false;

    while let Some(message) = stream.next().await {
        let message = message.expect("message should deserialize");

        match message {
            Message::Response(envelope) => match envelope.response {
                Response::GeneratedToken(token_result) => match token_result {
                    GeneratedTokenResult::Token(_) => {
                        received_tokens = true;
                    }
                    GeneratedTokenResult::Done => break,
                    GeneratedTokenResult::ChatTemplateError(error) => {
                        panic!("chat template error: {error}");
                    }
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
        "should have received tokens proving the chat template rendered the prompt"
    );
}

#[tokio::test]
#[file_serial]
async fn test_agent_can_use_chat_template_for_model() {
    let chat_template = ChatTemplate {
        content: SIMPLE_CHAT_TEMPLATE.to_string(),
    };

    let cluster = ManagedCluster::spawn(chat_template_cluster_params(
        AgentDesiredModel::HuggingFace(HuggingFaceModelReference {
            filename: "nomic-embed-text-v1.5.Q2_K.gguf".to_string(),
            repo_id: "nomic-ai/nomic-embed-text-v1.5-GGUF".to_string(),
            revision: "main".to_string(),
        }),
        chat_template.clone(),
    ))
    .await
    .expect("failed to spawn cluster");

    assert_agent_uses_chat_template_override(&cluster.balancer).await;

    let agent_id = get_first_agent_id(&cluster.balancer).await;

    let retrieved_template = cluster
        .balancer
        .client()
        .management()
        .get_chat_template_override(&agent_id)
        .await
        .expect("failed to get chat template override");

    assert_eq!(
        retrieved_template,
        Some(chat_template),
        "agent should have the provided chat template override"
    );

    assert_chat_template_renders_for_inference(&cluster.balancer).await;
}

#[tokio::test]
#[file_serial]
async fn test_agent_overrides_chat_template() {
    let chat_template = ChatTemplate {
        content: SIMPLE_CHAT_TEMPLATE.to_string(),
    };

    let cluster = ManagedCluster::spawn(chat_template_cluster_params(
        AGENT_DESIRED_MODEL.clone(),
        chat_template.clone(),
    ))
    .await
    .expect("failed to spawn cluster");

    assert_agent_uses_chat_template_override(&cluster.balancer).await;

    let agent_id = get_first_agent_id(&cluster.balancer).await;

    let retrieved_template = cluster
        .balancer
        .client()
        .management()
        .get_chat_template_override(&agent_id)
        .await
        .expect("failed to get chat template override");

    assert_eq!(
        retrieved_template,
        Some(chat_template),
        "agent should have the override template instead of the built-in one"
    );

    assert_chat_template_renders_for_inference(&cluster.balancer).await;
}
