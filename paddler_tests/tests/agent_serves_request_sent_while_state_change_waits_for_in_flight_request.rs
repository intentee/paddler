#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use anyhow::Context as _;
use anyhow::Result;
use futures_util::StreamExt as _;
use paddler_messaging::agent_state_application_status::AgentStateApplicationStatus;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::grammar_constraint::GrammarConstraint;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::collect_generated_tokens::collect_generated_tokens;
use paddler_test_cluster_harness::collected_generated_tokens::CollectedGeneratedTokens;
use paddler_test_cluster_harness::observation_window::ObservationWindow;
use paddler_tests::desired_state_with_halved_image_resize::desired_state_with_halved_image_resize;
use paddler_tests::qwen3_desired_state::qwen3_desired_state;
use paddler_tests::start_cluster::start_cluster;
use tokio_util::sync::CancellationToken;

const NEVER_COMPLETING_GRAMMAR: &str = r#"root ::= "apple " root"#;
const IN_FLIGHT_REQUEST_MAX_TOKENS: NonZeroU32 = NonZeroU32::new(512).unwrap();

fn finished_with_done(collected: &CollectedGeneratedTokens) -> bool {
    matches!(
        collected
            .token_results
            .last()
            .map(|token_result_with_producer| &token_result_with_producer.token_result),
        Some(GeneratedTokenResult::Done(_))
    )
}

#[tokio::test(flavor = "multi_thread")]
async fn agent_serves_request_sent_while_state_change_waits_for_in_flight_request() -> Result<()> {
    let initial_desired_state = qwen3_desired_state();
    let mut cluster = start_cluster(ClusterParams {
        agents: AgentConfig::uniform(1, 2),
        desired_state: Some(initial_desired_state.clone()),
        wait_for_slots_ready: true,
        ..ClusterParams::without_request_expiry()
    })
    .await?;

    let agent_id = cluster
        .agent_ids
        .first()
        .context("cluster must have one registered agent")?
        .clone();

    let mut in_flight_stream = cluster
        .continue_from_raw_prompt_stream(
            CancellationToken::new(),
            &ContinueFromRawPromptParams {
                grammar: Some(GrammarConstraint::Gbnf {
                    grammar: NEVER_COMPLETING_GRAMMAR.to_owned(),
                    root: "root".to_owned(),
                }),
                max_tokens: IN_FLIGHT_REQUEST_MAX_TOKENS,
                raw_prompt: "Repeat the word apple.".to_owned(),
            },
        )
        .await?;

    in_flight_stream
        .next()
        .await
        .context("the in-flight request must stream a first token")??;

    cluster
        .client_management
        .put_balancer_desired_state(
            CancellationToken::new(),
            &desired_state_with_halved_image_resize(initial_desired_state)?,
        )
        .await?;

    cluster
        .agents_watcher
        .until_agent(&agent_id, ObservationWindow::release(), |snapshot| {
            snapshot
                .agents
                .iter()
                .any(|agent| agent.state_application_status != AgentStateApplicationStatus::Applied)
        })
        .await
        .context("the agent must start applying the changed state")?;

    let (in_flight_collected, sent_during_state_change_collected) = tokio::join!(
        collect_generated_tokens(in_flight_stream),
        cluster.continue_from_raw_prompt(
            CancellationToken::new(),
            &ContinueFromRawPromptParams {
                grammar: None,
                max_tokens: NonZeroU32::new(4).unwrap(),
                raw_prompt: "Hello".to_owned(),
            },
        )
    );

    assert!(finished_with_done(&in_flight_collected?));
    assert!(finished_with_done(&sent_during_state_change_collected?));

    cluster.shutdown().await?;

    Ok(())
}
