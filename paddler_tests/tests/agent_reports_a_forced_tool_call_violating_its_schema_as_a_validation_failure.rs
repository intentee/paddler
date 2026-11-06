#![cfg(feature = "tests_that_use_llms")]

use tokio_util::sync::CancellationToken;

use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_tests::gbnf_literal::gbnf_literal;
use paddler_tests::get_weather_tool::get_weather_tool;
use paddler_tests::max_tokens_for_forced_text::max_tokens_for_forced_text;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;
use paddler_tests::weather_question_conversation::weather_question_conversation;
use paddler_tool_call_validator::tool_call_validation_error::ToolCallValidationError;

const TOOL_CALL_WITH_A_NUMBER_FOR_ITS_LOCATION: &str =
    "<tool_call>\n{\"name\": \"get_weather\", \"arguments\": {\"location\": 42}}\n</tool_call>";

#[tokio::test(flavor = "multi_thread")]
async fn agent_reports_a_forced_tool_call_violating_its_schema_as_a_validation_failure() {
    let cluster = start_cluster_with_qwen3(AgentConfig::uniform(1, 1))
        .await
        .expect("the cluster must start");

    let token_results = cluster
        .continue_from_conversation_history(
            CancellationToken::new(),
            &weather_question_conversation(
                gbnf_literal(TOOL_CALL_WITH_A_NUMBER_FOR_ITS_LOCATION),
                max_tokens_for_forced_text(TOOL_CALL_WITH_A_NUMBER_FOR_ITS_LOCATION),
                vec![get_weather_tool()],
            ),
        )
        .await
        .expect("the inference request must be accepted")
        .into_token_results();

    assert_eq!(
        token_results
            .iter()
            .filter(|token_result| token_result.is_tool_call_parsed()
                || token_result.is_tool_call_failure())
            .collect::<Vec<_>>(),
        vec![&GeneratedTokenResult::ToolCallValidationFailed(vec![
            ToolCallValidationError::SchemaMismatch {
                tool_name: "get_weather".to_owned(),
                violations: vec![r#"42 is not of type "string""#.to_owned()],
            }
            .to_string()
        ])]
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
