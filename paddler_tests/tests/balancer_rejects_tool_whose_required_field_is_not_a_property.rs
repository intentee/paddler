use paddler_client::error::Error as ClientError;
use paddler_messaging::conversation_history::ConversationHistory;
use paddler_messaging::conversation_message::ConversationMessage;
use paddler_messaging::conversation_message_content::ConversationMessageContent;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::Tool;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::FunctionCall;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::function::Function;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters::Parameters;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::start_cluster::start_cluster;
use serde_json::Map;
use serde_json::json;
use tokio_util::sync::CancellationToken;

const BAD_REQUEST: u16 = 400;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_rejects_tool_whose_required_field_is_not_a_property() {
    let cluster = start_cluster(ClusterParams {
        agents: Vec::new(),
        wait_for_slots_ready: false,
        ..ClusterParams::default()
    })
    .await
    .expect("a cluster without agents must start");
    let mut name_properties = Map::new();

    name_properties.insert("name".to_owned(), json!({"type": "string"}));

    let rejection = cluster
        .client_inference
        .post_continue_from_conversation_history(
            CancellationToken::new(),
            &ContinueFromConversationHistoryParams {
                add_generation_prompt: true,
                conversation_history: ConversationHistory::new(vec![ConversationMessage {
                    content: ConversationMessageContent::Text("Say hello".to_owned()),
                    role: "user".to_owned(),
                }]),
                enable_thinking: true,
                grammar: None,
                max_tokens: 10,
                parse_tool_calls: true,
                tools: vec![Tool::Function(FunctionCall {
                    function: Function {
                        name: "test_fn".to_owned(),
                        description: "test".to_owned(),
                        parameters: Parameters::Schema(ValidatedParametersSchema {
                            schema_type: "object".to_owned(),
                            properties: Some(name_properties),
                            required: Some(vec!["nonexistent_field".to_owned()]),
                            additional_properties: None,
                        }),
                    },
                })],
            },
        )
        .await
        .err()
        .expect("the balancer must reject a tool whose required field is not a property");

    assert!(matches!(
        rejection,
        ClientError::UnexpectedResponseStatus { message, status, .. }
            if status.as_u16() == BAD_REQUEST
                && message == "Invalid request parameters: Required field 'nonexistent_field' not found in properties"
    ));

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
