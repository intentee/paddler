#![cfg(feature = "tests_that_use_llms")]

use anyhow::Context as _;
use anyhow::Result;
use futures_util::StreamExt as _;
use paddler_messaging::agent_state_application_status::AgentStateApplicationStatus;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::grammar_constraint::GrammarConstraint;
use paddler_messaging::inference_parameters::InferenceParameters;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::observation_window::ObservationWindow;
use paddler_tests::qwen3_desired_state::qwen3_desired_state;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;
use tokio_util::sync::CancellationToken;

const NEVER_COMPLETING_GRAMMAR: &str = r#"root ::= "apple " root"#;

#[tokio::test(flavor = "multi_thread")]
async fn agent_shuts_down_while_state_change_waits_for_in_flight_request() -> Result<()> {
    let mut cluster = start_cluster_with_qwen3(AgentConfig::uniform(1, 1)).await?;
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
                max_tokens: i32::MAX,
                raw_prompt: "Repeat the word apple.".to_owned(),
            },
        )
        .await?;

    in_flight_stream
        .next()
        .await
        .context("the in-flight request must stream a first token")??;

    let initial_desired_state = qwen3_desired_state();

    cluster
        .client_management
        .put_balancer_desired_state(
            CancellationToken::new(),
            &BalancerDesiredState {
                inference_parameters: InferenceParameters {
                    image_resize_to_fit: initial_desired_state
                        .inference_parameters
                        .image_resize_to_fit
                        / 2,
                    ..initial_desired_state.inference_parameters.clone()
                },
                ..initial_desired_state
            },
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

    cluster.shutdown().await
}
