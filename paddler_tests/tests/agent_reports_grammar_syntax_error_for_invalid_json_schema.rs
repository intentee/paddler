#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use paddler_cluster::agent_config::AgentConfig;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::grammar_constraint::GrammarConstraint;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;

#[tokio::test(flavor = "multi_thread")]
async fn agent_reports_grammar_syntax_error_for_invalid_json_schema() -> Result<()> {
    let cluster = start_cluster_with_qwen3(AgentConfig::uniform(1, 2)).await?;

    let collected = cluster
        .inference_client
        .http()
        .continue_from_raw_prompt_collected(&ContinueFromRawPromptParams {
            grammar: Some(GrammarConstraint::JsonSchema {
                schema: "not valid json".to_owned(),
            }),
            max_tokens: 10,
            raw_prompt: "Say hi.".to_owned(),
        })
        .await?;

    let reported_grammar_syntax_error = collected.token_results.iter().any(|event| {
        matches!(
            event.token_result,
            GeneratedTokenResult::GrammarSyntaxError(_)
        )
    });

    assert!(reported_grammar_syntax_error);

    cluster.shutdown().await?;

    Ok(())
}
