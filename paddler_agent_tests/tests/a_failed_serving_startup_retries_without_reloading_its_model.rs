#![cfg(feature = "tests_that_use_llms")]

use std::mem::discriminant;

use paddler_agent_pipeline_harness::desired_state_reconciled_discriminant::desired_state_reconciled_discriminant;
use paddler_agent_pipeline_harness::serving_pipeline_arbiter::ServingPipelineArbiter;
use paddler_agent_tests::swappable_model_file::SwappableModelFile;
use paddler_balancer_desired_state_fixture::desired_state_with_chat_template_override::desired_state_with_chat_template_override;
use paddler_balancer_desired_state_fixture::uncompilable_chat_template::uncompilable_chat_template;
use paddler_balancer_desired_state_fixture::uncompilable_chat_template_issue::UncompilableChatTemplateIssue;
use paddler_messaging::agent_issue_params::model_path::ModelPath;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::produces_snapshot::ProducesSnapshot as _;
use paddler_model_card_fixture::qwen3_5_0_8b::qwen3_5_0_8b;

#[tokio::test(flavor = "multi_thread")]
async fn a_failed_serving_startup_retries_without_reloading_its_model() {
    let swappable_model_file = SwappableModelFile::link_to(
        &qwen3_5_0_8b()
            .cached_path()
            .expect("the Qwen3.5 model must be in the Hugging Face cache"),
    )
    .expect("the model must be linked into a swappable model file");
    let mut serving_pipeline_arbiter =
        ServingPipelineArbiter::new(1).expect("the model downloader must start");

    assert_eq!(
        discriminant(
            &serving_pipeline_arbiter
                .apply(desired_state_with_chat_template_override(
                    BalancerDesiredState {
                        model: swappable_model_file
                            .agent_desired_model()
                            .expect("the swappable model file must be at an absolute path"),
                        ..qwen3_5_0_8b().into_desired_state()
                    },
                    uncompilable_chat_template(),
                ))
                .await
        ),
        desired_state_reconciled_discriminant()
    );

    let failed_attempt = serving_pipeline_arbiter
        .pipeline_arbiter
        .next_finished_startup()
        .await;

    swappable_model_file
        .replace_with_an_unloadable_file()
        .expect("the unloadable file must replace the model");
    serving_pipeline_arbiter
        .pipeline_arbiter
        .conclude(failed_attempt);
    serving_pipeline_arbiter.conclude_next_startup().await;

    let issues = serving_pipeline_arbiter
        .inference_runtime_context
        .slot_aggregated_status
        .make_snapshot()
        .status
        .issues;
    let uncompilable_chat_template_issue = UncompilableChatTemplateIssue {
        model_path: ModelPath::from(swappable_model_file.model_path().as_path()),
    };

    assert_eq!(issues.len(), 1);
    assert!(
        issues
            .iter()
            .all(|issue| uncompilable_chat_template_issue.is_reported_by(issue))
    );

    serving_pipeline_arbiter.shut_down().await;
}
