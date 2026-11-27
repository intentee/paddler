#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU16;
use std::num::NonZeroU32;

use llama_cpp_context::context_error::ContextError;
use tokio_util::sync::CancellationToken;

use paddler_agent_decision::decision_error::DecisionError;
use paddler_agent_decision::decision_pipeline::DecisionPipeline;
use paddler_agent_pipeline_harness::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_agent_pipeline_harness::established_decision_capabilities::EstablishedDecisionCapabilities;
use paddler_agent_pipeline_harness::llama_runtime::LlamaRuntime;
use paddler_agent_runtime::agent_runtime_error::AgentRuntimeError;
use paddler_inference_parameters::desired_model_runtime_parameters::DesiredModelRuntimeParameters;
use paddler_inference_parameters::model_default_or::ModelDefaultOr;
use paddler_model_card_fixture::kev_0_8b::kev_0_8b;

const SLOTS: NonZeroU16 = NonZeroU16::new(200).unwrap();
const CACHE_CELLS_FEWER_THAN_TWO_PER_SLOT: NonZeroU32 = NonZeroU32::new(256).unwrap();

#[tokio::test(flavor = "multi_thread")]
async fn a_context_without_cells_for_every_slot_fails_its_warm_up_decode() {
    let llama_runtime = LlamaRuntime::load(
        &kev_0_8b()
            .cached_path()
            .expect("the kev model must be in the Hugging Face cache"),
        agent_inference_runtime_context(SLOTS.get()),
    )
    .expect("the kev model must load");
    let EstablishedDecisionCapabilities {
        decision_model,
        model_capabilities,
    } = EstablishedDecisionCapabilities::of(&llama_runtime)
        .expect("llama.cpp must serve decisions of the kev model");
    let LlamaRuntime {
        inference_runtime_context,
        loaded_llama_model,
        model_runtime_parameters,
    } = llama_runtime;

    assert!(matches!(
        DecisionPipeline {
            decision_model,
            decision_slots: SLOTS,
            inference_runtime_context,
            loaded_llama_model,
            model_capabilities,
            model_runtime_parameters: DesiredModelRuntimeParameters {
                context_size: ModelDefaultOr::Explicit(CACHE_CELLS_FEWER_THAN_TWO_PER_SLOT),
                ..model_runtime_parameters
            },
        }
        .spawn(&CancellationToken::new())
        .await,
        Err(DecisionError::AgentRuntime(
            AgentRuntimeError::WarmUpDecodeFailed(ContextError::DecodeNoKvCacheSlot)
        ))
    ));
}
