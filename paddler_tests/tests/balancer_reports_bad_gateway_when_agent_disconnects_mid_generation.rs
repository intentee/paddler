#![cfg(feature = "tests_that_use_llms")]

use anyhow::Context as _;
use anyhow::Result;
use anyhow::bail;
use futures_util::StreamExt as _;
use paddler_messaging::grammar_constraint::GrammarConstraint;
use paddler_messaging::inference_client::message::Message;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;
use tokio_util::sync::CancellationToken;

const BAD_GATEWAY: i32 = 502;
const NEVER_COMPLETING_GRAMMAR: &str = r#"root ::= "apple " root"#;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_reports_bad_gateway_when_agent_disconnects_mid_generation() -> Result<()> {
    let mut cluster = start_cluster_with_qwen3(AgentConfig::uniform(1, 1)).await?;

    let mut stream = cluster
        .continue_from_raw_prompt_stream(
            CancellationToken::new(),
            &ContinueFromRawPromptParams {
                grammar: Some(GrammarConstraint::Gbnf {
                    grammar: NEVER_COMPLETING_GRAMMAR.to_owned(),
                    root: "root".to_owned(),
                }),
                max_tokens: i32::MAX,
                raw_prompt: "Repeat the word apple.".to_owned(),
            },
        )
        .await?;

    stream
        .next()
        .await
        .context("the request must stream a first token")??;

    cluster
        .agents
        .pop()
        .context("cluster must have one running agent")?
        .shutdown()
        .await?;

    while let Some(message) = stream.next().await {
        if let Message::Error(error_envelope) = message? {
            assert_eq!(error_envelope.error.code, BAD_GATEWAY);

            cluster.shutdown().await?;

            return Ok(());
        }
    }

    bail!("the stream ended without reporting the lost agent")
}
