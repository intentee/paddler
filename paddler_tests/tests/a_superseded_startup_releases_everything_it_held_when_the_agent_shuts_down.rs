#![cfg(feature = "tests_that_use_llms")]

use std::mem::discriminant;

use paddler_agent_runtime::agent_vram_residencies::AgentVramResidencies;
use paddler_inference_parameters::embedding_parameters::EmbeddingParameters;
use paddler_inference_parameters::pooling_type::PoolingType;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::decision_settings::DecisionSettings;
use paddler_tests::desired_state_reconciled_discriminant::desired_state_reconciled_discriminant;
use paddler_tests::kev_checkpoint::KevCheckpoint;

use crate::serving_pipeline_arbiter::ServingPipelineArbiter;

#[tokio::test(flavor = "multi_thread")]
async fn a_superseded_startup_releases_everything_it_held_when_the_agent_shuts_down() {
    let superseded_desired_state = BalancerDesiredState {
        decision: DecisionSettings::default(),
        ..KevCheckpoint::Kev0_8b
            .desired_state()
            .expect("the kev desired state must name absolute paths")
    };
    let mut serving_pipeline_arbiter = ServingPipelineArbiter::new(1);
    let AgentVramResidencies {
        decisions_context,
        embeddings_context,
        model_weights,
        multimodal_projection,
        text_generation_context,
    } = serving_pipeline_arbiter
        .inference_runtime_context
        .vram_residencies
        .clone();

    assert_eq!(
        discriminant(
            &serving_pipeline_arbiter
                .apply(superseded_desired_state.clone())
                .await
        ),
        desired_state_reconciled_discriminant()
    );
    serving_pipeline_arbiter.conclude_next_progress().await;
    assert_eq!(
        discriminant(
            &serving_pipeline_arbiter
                .apply(BalancerDesiredState {
                    embeddings: EmbeddingParameters {
                        pooling_type: PoolingType::Mean,
                        ..superseded_desired_state.embeddings.clone()
                    },
                    ..superseded_desired_state
                })
                .await
        ),
        desired_state_reconciled_discriminant()
    );
    serving_pipeline_arbiter.conclude_next_progress().await;
    serving_pipeline_arbiter.conclude_next_progress().await;
    serving_pipeline_arbiter.conclude_next_progress().await;
    serving_pipeline_arbiter.shut_down().await;

    assert!(
        [
            decisions_context,
            embeddings_context,
            model_weights,
            multimodal_projection,
            text_generation_context,
        ]
        .iter()
        .all(|vram_residency| vram_residency.try_occupy().is_ok())
    );
}
