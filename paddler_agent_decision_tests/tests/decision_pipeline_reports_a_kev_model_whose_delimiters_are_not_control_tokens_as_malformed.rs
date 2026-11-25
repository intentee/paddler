#![cfg(feature = "tests_that_use_llms")]

use paddler_agent_pipeline_harness::model_declaring_decision_metadata::ModelDeclaringDecisionMetadata;
use paddler_agent_pipeline_harness::serving_pipeline_arbiter::ServingPipelineArbiter;
use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::agent_issue_params::model_path::ModelPath;
use paddler_messaging::constants::DECISION_SLOTS_MINIMUM;
use paddler_model_card_fixture::smolvlm2_256m::smolvlm2_256m;

#[tokio::test(flavor = "multi_thread")]
async fn decision_pipeline_reports_a_kev_model_whose_delimiters_are_not_control_tokens_as_malformed()
 {
    let kev_model_with_another_vocabulary =
        ModelDeclaringDecisionMetadata::write(smolvlm2_256m(), |gguf_context| {
            gguf_context.set_val_str(c"llama.decision.type", c"kev");
            gguf_context.set_val_str(c"llama.decision.temperature.choice", c"1.0");
            gguf_context.set_val_str(c"llama.decision.temperature.noul", c"1.0");
            gguf_context.set_val_str(c"llama.decision.temperature.score", c"1.0");

            Ok(())
        })
        .expect("the model declaring a kev decision type must be written");
    let mut serving_pipeline_arbiter = ServingPipelineArbiter::new(DECISION_SLOTS_MINIMUM.get())
        .expect("the model downloader must start");

    serving_pipeline_arbiter
        .serve(
            kev_model_with_another_vocabulary
                .desired_state()
                .expect("the decision model must lie at an absolute path"),
        )
        .await
        .expect("the desired state must be reconciled");

    assert!(
        serving_pipeline_arbiter
            .inference_runtime_context
            .slot_aggregated_status
            .has_issue(&AgentIssue::DecisionModelIsMalformed(ModelPath::from(
                kev_model_with_another_vocabulary.model_path().as_path()
            )))
    );

    serving_pipeline_arbiter.shut_down().await;
}
