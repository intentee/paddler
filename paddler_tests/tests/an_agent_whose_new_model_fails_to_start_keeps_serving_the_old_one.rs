#![cfg(feature = "tests_that_use_llms")]

use std::io::Write as _;
use std::num::NonZeroU32;

use tempfile::NamedTempFile;
use tokio_util::sync::CancellationToken;

use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::generation_finish::GenerationFinish;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_model_source::model_source::ModelSource;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::unending_generation::unending_generation;
use paddler_tests::qwen3_desired_state::qwen3_desired_state;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;

#[tokio::test(flavor = "multi_thread")]
async fn an_agent_whose_new_model_fails_to_start_keeps_serving_the_old_one() {
    let mut unloadable_model =
        NamedTempFile::new().expect("a temporary model file must be creatable");

    unloadable_model
        .write_all(b"this is not a valid gguf model file")
        .expect("the unloadable model contents must be writable");

    let unloadable_model_path = unloadable_model
        .path()
        .to_str()
        .expect("the temporary model path must be valid UTF-8")
        .to_owned();
    let mut cluster = start_cluster_with_qwen3(vec![AgentConfig::single(1)])
        .await
        .expect("the cluster must start");

    cluster
        .client_management
        .put_balancer_desired_state(
            CancellationToken::new(),
            &BalancerDesiredState {
                model: ModelSource::LocalToAgent(unloadable_model_path.clone())
                    .into_agent_desired_model(),
                ..qwen3_desired_state()
            },
        )
        .await
        .expect("the balancer must accept the desired state");
    cluster
        .wait_for_first_agent_issue(|issue| {
            matches!(issue, AgentIssue::ModelCannotBeLoaded(model_path) if model_path.model_path == unloadable_model_path)
        })
        .await
        .expect("the agent must report that the new model cannot be loaded");

    assert_eq!(
        cluster
            .continue_from_raw_prompt(
                CancellationToken::new(),
                &ContinueFromRawPromptParams {
                    max_tokens: NonZeroU32::MIN,
                    ..unending_generation()
                },
            )
            .await
            .expect("the old model must answer the request")
            .summary()
            .expect("the generation must finish with a summary")
            .finish,
        GenerationFinish::MaxTokens
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
