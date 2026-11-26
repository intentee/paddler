#![cfg(feature = "tests_that_use_llms")]

use std::collections::BTreeSet;

use llama_cpp_bindings_types::model_capabilities::ModelCapabilities;
use llama_cpp_bindings_types::text_generation_capability::TextGenerationCapability;
use llama_cpp_bindings_types::text_generation_unsupported_reason::TextGenerationUnsupportedReason;

use paddler_agent_pipeline_harness::serving_pipeline_arbiter::ServingPipelineArbiter;
use paddler_inference_parameters::desired_model_runtime_parameters::DesiredModelRuntimeParameters;
use paddler_inference_parameters::model_default_or::ModelDefaultOr;
use paddler_messaging::agent_runtime_status::AgentRuntimeStatus;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::produces_snapshot::ProducesSnapshot as _;
use paddler_model_card_fixture::t5_small::t5_small;

const SEVERAL_SLOTS: u16 = 2;

#[tokio::test(flavor = "multi_thread")]
async fn an_encoder_decoder_model_fitted_for_several_slots_reports_that_it_generates_no_text() {
    let t5_small_desired_state = t5_small().into_desired_state();
    let mut serving_pipeline_arbiter =
        ServingPipelineArbiter::new(SEVERAL_SLOTS).expect("the model downloader must start");

    serving_pipeline_arbiter
        .serve(BalancerDesiredState {
            model_runtime_parameters: DesiredModelRuntimeParameters {
                context_size: ModelDefaultOr::ModelDefault,
                ..t5_small_desired_state.model_runtime_parameters
            },
            ..t5_small_desired_state
        })
        .await
        .expect("the desired state must be served");

    let status = serving_pipeline_arbiter
        .inference_runtime_context
        .slot_aggregated_status
        .make_snapshot()
        .status;

    assert_eq!(status.issues, BTreeSet::new());
    assert!(matches!(
        status.runtime,
        AgentRuntimeStatus::Serving {
            capabilities: ModelCapabilities {
                text_generation: TextGenerationCapability::Unsupported(
                    TextGenerationUnsupportedReason::EncoderDecoderArchitecture
                ),
                ..
            },
            ..
        }
    ));

    serving_pipeline_arbiter.shut_down().await;
}
