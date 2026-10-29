#![cfg(feature = "tests_that_use_llms")]

use std::collections::BTreeSet;
use std::mem::discriminant;

use paddler_inference_parameters::embedding_parameters::EmbeddingParameters;
use paddler_inference_parameters::pooling_type::PoolingType;
use paddler_messaging::agent_decisions_serving::AgentDecisionsServing;
use paddler_messaging::agent_runtime_status::AgentRuntimeStatus;
use paddler_messaging::agent_state_application_status::AgentStateApplicationStatus;
use paddler_messaging::agent_status::AgentStatus;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::decision_settings::DecisionSettings;
use paddler_messaging::model_download_status::ModelDownloadStatus;
use paddler_messaging::produces_snapshot::ProducesSnapshot as _;
use paddler_tests::desired_state_with_chat_template_override::desired_state_with_chat_template_override;
use paddler_tests::first_message_content_chat_template::first_message_content_chat_template;
use paddler_tests::kev_checkpoint::KevCheckpoint;

use crate::serving_pipeline_arbiter::ServingPipelineArbiter;
use crate::swappable_model_file::SwappableModelFile;
use paddler_tests::desired_state_reconciled_discriminant::desired_state_reconciled_discriminant;

#[tokio::test(flavor = "multi_thread")]
async fn a_reconfiguration_replaced_before_it_starts_yields_to_its_replacement() {
    let swappable_model_file =
        SwappableModelFile::link_to(&KevCheckpoint::Kev0_8b.target_path("model.gguf"));
    let served_desired_state = BalancerDesiredState {
        decision: DecisionSettings::default(),
        model: swappable_model_file.agent_desired_model(),
        ..KevCheckpoint::Kev0_8b
            .desired_state()
            .expect("the kev desired state must name absolute paths")
    };
    let mut serving_pipeline_arbiter = ServingPipelineArbiter::new(1);

    serving_pipeline_arbiter
        .serve(served_desired_state.clone())
        .await;

    swappable_model_file.replace_with_an_unloadable_file();

    assert_eq!(
        discriminant(
            &serving_pipeline_arbiter
                .apply(BalancerDesiredState {
                    embeddings: EmbeddingParameters {
                        pooling_type: PoolingType::Mean,
                        ..served_desired_state.embeddings.clone()
                    },
                    ..served_desired_state.clone()
                })
                .await
        ),
        desired_state_reconciled_discriminant()
    );
    assert_eq!(
        discriminant(
            &serving_pipeline_arbiter
                .apply(desired_state_with_chat_template_override(
                    served_desired_state,
                    first_message_content_chat_template(),
                ))
                .await
        ),
        desired_state_reconciled_discriminant()
    );
    serving_pipeline_arbiter.conclude_next_progress().await;
    serving_pipeline_arbiter.conclude_next_progress().await;
    serving_pipeline_arbiter.conclude_next_progress().await;
    assert_eq!(
        serving_pipeline_arbiter
            .inference_runtime_context
            .slot_aggregated_status
            .make_snapshot()
            .status,
        AgentStatus {
            desired_slots_total: 1,
            download_status: ModelDownloadStatus::NotDownloading,
            issues: BTreeSet::new(),
            model_path: Some(swappable_model_file.model_path().display().to_string()),
            runtime: AgentRuntimeStatus::Serving {
                decisions: AgentDecisionsServing::NotServed,
                slots_total: 1,
            },
            state_application_status: AgentStateApplicationStatus::Applied,
            uses_chat_template_override: true,
        }
    );

    serving_pipeline_arbiter.shut_down().await;
}
