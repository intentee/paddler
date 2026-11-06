#![cfg(feature = "tests_that_use_llms")]

use std::collections::BTreeSet;
use std::mem::discriminant;

use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::decision_settings::DecisionSettings;
use paddler_messaging::produces_snapshot::ProducesSnapshot as _;
use paddler_tests::desired_state_reconciled_discriminant::desired_state_reconciled_discriminant;
use paddler_tests::desired_state_with_chat_template_override::desired_state_with_chat_template_override;
use paddler_tests::kev_checkpoint::KevCheckpoint;
use paddler_tests::uncompilable_chat_template::uncompilable_chat_template;
use paddler_tests::uncompilable_chat_template_issue::uncompilable_chat_template_issue;

use crate::serving_pipeline_arbiter::ServingPipelineArbiter;
use crate::swappable_model_file::SwappableModelFile;

#[tokio::test(flavor = "multi_thread")]
async fn a_failed_attempt_retries_without_reloading_its_model() {
    let swappable_model_file =
        SwappableModelFile::link_to(&KevCheckpoint::Kev0_8b.target_path("model.gguf"));
    let mut serving_pipeline_arbiter = ServingPipelineArbiter::new(1);

    assert_eq!(
        discriminant(
            &serving_pipeline_arbiter
                .apply(desired_state_with_chat_template_override(
                    BalancerDesiredState {
                        decision: DecisionSettings::default(),
                        model: swappable_model_file.agent_desired_model(),
                        ..KevCheckpoint::Kev0_8b
                            .decision_desired_state()
                            .into_balancer_desired_state()
                            .expect("the kev desired state must name absolute paths")
                    },
                    uncompilable_chat_template(),
                ))
                .await
        ),
        desired_state_reconciled_discriminant()
    );

    serving_pipeline_arbiter.conclude_next_progress().await;

    let failed_attempt = serving_pipeline_arbiter
        .pipeline_arbiter
        .next_finished_startup()
        .await;

    swappable_model_file.replace_with_an_unloadable_file();
    serving_pipeline_arbiter
        .pipeline_arbiter
        .conclude(failed_attempt);
    serving_pipeline_arbiter.conclude_next_progress().await;
    serving_pipeline_arbiter.conclude_next_progress().await;

    assert_eq!(
        serving_pipeline_arbiter
            .inference_runtime_context
            .slot_aggregated_status
            .make_snapshot()
            .status
            .issues,
        BTreeSet::from([uncompilable_chat_template_issue(
            swappable_model_file.model_path().as_path()
        )])
    );

    serving_pipeline_arbiter.shut_down().await;
}
