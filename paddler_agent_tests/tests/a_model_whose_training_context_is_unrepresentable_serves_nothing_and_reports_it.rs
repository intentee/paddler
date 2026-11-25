#![cfg(feature = "tests_that_use_llms")]

use paddler_agent_pipeline_harness::model_with_patched_metadata::ModelWithPatchedMetadata;
use paddler_agent_pipeline_harness::serving_pipeline_arbiter::ServingPipelineArbiter;
use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::agent_issue_params::model_path::ModelPath;
use paddler_messaging::agent_runtime_status::AgentRuntimeStatus;
use paddler_messaging::produces_snapshot::ProducesSnapshot as _;
use paddler_model_card_fixture::smolvlm2_256m::smolvlm2_256m;

#[tokio::test(flavor = "multi_thread")]
async fn a_model_whose_training_context_is_unrepresentable_serves_nothing_and_reports_it() {
    let model_with_an_unrepresentable_training_context =
        ModelWithPatchedMetadata::write(smolvlm2_256m(), |gguf_context| {
            gguf_context.set_val_u32(c"llama.context_length", u32::MAX);

            Ok(())
        })
        .expect("the model with an unrepresentable training context must be written");
    let mut serving_pipeline_arbiter =
        ServingPipelineArbiter::new(1).expect("the model downloader must start");

    serving_pipeline_arbiter
        .apply(
            model_with_an_unrepresentable_training_context
                .desired_state()
                .expect("the model must lie at an absolute path"),
        )
        .await;
    serving_pipeline_arbiter.conclude_next_startup().await;

    let status = serving_pipeline_arbiter
        .inference_runtime_context
        .slot_aggregated_status
        .make_snapshot()
        .status;

    assert!(
        status
            .issues
            .contains(&AgentIssue::ModelTrainingContextIsUnrepresentable(
                ModelPath::from(
                    model_with_an_unrepresentable_training_context
                        .model_path()
                        .as_path()
                )
            ))
    );
    assert_eq!(status.runtime, AgentRuntimeStatus::Idle);

    serving_pipeline_arbiter.shut_down().await;
}
