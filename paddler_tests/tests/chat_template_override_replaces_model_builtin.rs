#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use tokio_util::sync::CancellationToken;

use paddler_inference_parameters::all_gpu_layers::ALL_GPU_LAYERS;
use paddler_inference_parameters::model_runtime_parameters::ModelRuntimeParameters;
use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::chat_template::ChatTemplate;
use paddler_messaging::conversation_history::ConversationHistory;
use paddler_messaging::conversation_message::ConversationMessage;
use paddler_messaging::conversation_message_content::ConversationMessageContent;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::model_card::ModelCard;
use paddler_test_cluster_harness::model_card::qwen3_0_6b::qwen3_0_6b;
use paddler_tests::desired_state_with_chat_template_override::desired_state_with_chat_template_override;
use paddler_tests::start_cluster::start_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn chat_template_override_replaces_model_builtin() {
    let ModelCard { reference } = qwen3_0_6b();

    let chat_template = ChatTemplate {
        content: "{{ messages[0].content }}".to_owned(),
    };

    let cluster = start_cluster(ClusterParams {
        agents: AgentConfig::uniform(1, 1),
        wait_for_slots_ready: true,
        desired_state: ClusterDesiredState::Apply(Box::new(
            desired_state_with_chat_template_override(
                BalancerDesiredState {
                    model_runtime_parameters: ModelRuntimeParameters {
                        n_gpu_layers: ALL_GPU_LAYERS,
                        ..ModelRuntimeParameters::default()
                    },
                    model: AgentDesiredModel::HuggingFace(reference),
                    ..BalancerDesiredState::default()
                },
                chat_template.clone(),
            ),
        )),
        ..ClusterParams::default()
    })
    .await
    .expect("the cluster must start");

    let agent_id = cluster
        .agent_ids
        .first()
        .expect("cluster must have one registered agent")
        .clone();

    let retrieved = cluster
        .client_management
        .get_chat_template_override(CancellationToken::new(), &agent_id)
        .await
        .expect("failed to read chat template override");

    assert_eq!(retrieved, Some(chat_template));

    let collected = cluster
        .continue_from_conversation_history(
            CancellationToken::new(),
            &ContinueFromConversationHistoryParams {
                add_generation_prompt: true,
                conversation_history: ConversationHistory::new(vec![ConversationMessage {
                    content: ConversationMessageContent::Text(
                        "The capital of France is".to_owned(),
                    ),
                    role: "user".to_owned(),
                }]),
                enable_thinking: false,
                grammar: None,
                max_tokens: NonZeroU32::new(10).unwrap(),
                parse_tool_calls: false,
                tools: vec![],
            },
        )
        .await
        .expect("the inference request must be accepted");

    let received_tokens = collected
        .token_results
        .iter()
        .any(|result| result.token_result.is_token());

    assert!(
        received_tokens,
        "override template should render the prompt and produce tokens"
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
