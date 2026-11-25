#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use llama_cpp_context::llama_batch::LlamaBatch;
use llama_cpp_model::add_bos::AddBos;
use llama_cpp_model::parse_special_tokens::ParseSpecialTokens;
use log::LevelFilter;
use log::set_max_level;
use tokio::sync::mpsc::unbounded_channel;
use tokio_util::sync::CancellationToken;

use paddler_agent_pipeline_harness::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_agent_pipeline_harness::llama_runtime::LlamaRuntime;
use paddler_agent_pipeline_harness::raw_prompt_generation::RawPromptGeneration;
use paddler_agent_runtime::scheduler_message::SchedulerMessage;
use paddler_agent_text_generation::continuous_batch_scheduler::ContinuousBatchScheduler;
use paddler_agent_text_generation::continuous_batch_scheduler_context::ContinuousBatchSchedulerContext;
use paddler_agent_text_generation::continuous_batch_scheduler_params::ContinuousBatchSchedulerParams;
use paddler_agent_text_generation::multimodal_ingestion::MultimodalIngestion;
use paddler_agent_text_generation_tests::first_message_content_request_preparer::FirstMessageContentRequestPreparer;
use paddler_agent_text_generation_tests::last_generated_token_result::last_generated_token_result;
use paddler_inference_parameters::model_default_or::ModelDefaultOr;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_model_card_fixture::qwen3_5_0_8b::qwen3_5_0_8b;

const CACHE_CELLS: NonZeroU32 = NonZeroU32::new(256).unwrap();
const SENTENCE: &str = "Paddler balances inference across agents. ";

#[test]
fn a_generation_whose_prompt_the_kv_cache_cannot_hold_fails_as_a_decode_failure() {
    set_max_level(LevelFilter::Error);

    let mut runtime = LlamaRuntime::load(
        &qwen3_5_0_8b()
            .cached_path()
            .expect("the Qwen3.5 model must be in the Hugging Face cache"),
        agent_inference_runtime_context(1),
    )
    .expect("the Qwen3.5 model must load");

    runtime.model_runtime_parameters.context_size = ModelDefaultOr::Explicit(CACHE_CELLS);

    let llama_context = runtime
        .create_text_generation_llama_context(1)
        .expect("the context must be created");
    let cache_cells = llama_context.n_ctx_seq() as usize;
    let model = &runtime.loaded_llama_model.model;
    let mut prompt = SENTENCE.to_owned();

    while model
        .str_to_token(&prompt, AddBos::Always, ParseSpecialTokens::Always)
        .expect("the prompt must tokenize")
        .len()
        <= cache_cells
    {
        prompt.push_str(SENTENCE);
    }

    let sequence_context_declared_beyond_the_cache = u32::try_from(cache_cells * 2)
        .expect("twice the cache cells must fit a sequence context size");
    let request_preparer = FirstMessageContentRequestPreparer {
        llama_runtime: &runtime,
        sequence_context_size: sequence_context_declared_beyond_the_cache,
    }
    .into_generation_request_preparer()
    .expect("the first message content chat template must be prepared");
    let multimodal_ingestion = MultimodalIngestion::for_context(
        &llama_context,
        request_preparer.model_capabilities.clone(),
    );
    let RawPromptGeneration {
        agent_request,
        generate_tokens_stop_tx: _generate_tokens_stop_tx,
        generated_tokens_rx,
    } = RawPromptGeneration::new(
        &prompt,
        runtime
            .inference_runtime_context
            .slot_aggregated_status
            .clone(),
    );
    let (scheduler_message_tx, scheduler_message_rx) = unbounded_channel();

    scheduler_message_tx
        .send(SchedulerMessage::Command(
            request_preparer
                .prepare_raw_prompt(agent_request)
                .expect("the generation must be prepared"),
        ))
        .expect("the scheduler must accept the generation");
    scheduler_message_tx
        .send(SchedulerMessage::Shutdown)
        .expect("the scheduler must accept the shutdown");

    ContinuousBatchScheduler::new(ContinuousBatchSchedulerParams {
        agent_shutdown: CancellationToken::new(),
        batch: LlamaBatch::new(llama_context.limits()),
        llama_context,
        scheduler_context: ContinuousBatchSchedulerContext {
            agent_name: None,
            desired_slots_total: 1,
            model,
            multimodal_ingestion,
            sequence_context_size: sequence_context_declared_beyond_the_cache,
        },
        scheduler_message_rx,
    })
    .run();

    assert!(matches!(
        last_generated_token_result(generated_tokens_rx),
        Some(GeneratedTokenResult::DecodeFailed(_))
    ));
}
