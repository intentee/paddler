#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use llama_cpp_context::context_limits::ContextLimits;
use llama_cpp_context::llama_batch::LlamaBatch;
use tokio::sync::mpsc::unbounded_channel;
use tokio_util::sync::CancellationToken;

use paddler_agent_pipeline_harness::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_agent_pipeline_harness::conversation_generation::ConversationGeneration;
use paddler_agent_pipeline_harness::llama_runtime::LlamaRuntime;
use paddler_agent_runtime::scheduler_message::SchedulerMessage;
use paddler_agent_text_generation::continuous_batch_scheduler::ContinuousBatchScheduler;
use paddler_agent_text_generation::continuous_batch_scheduler_context::ContinuousBatchSchedulerContext;
use paddler_agent_text_generation::continuous_batch_scheduler_params::ContinuousBatchSchedulerParams;
use paddler_agent_text_generation::multimodal_ingestion::MultimodalIngestion;
use paddler_agent_text_generation_tests::first_message_content_request_preparer::FirstMessageContentRequestPreparer;
use paddler_agent_text_generation_tests::gbnf_literal::gbnf_literal;
use paddler_agent_text_generation_tests::last_generated_token_result::last_generated_token_result;
use paddler_agent_text_generation_tests::single_image_question::SingleImageQuestion;
use paddler_file_fixture::constants::LLAMAS_JPEG;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_model_card_fixture::smolvlm2_256m::smolvlm2_256m;
use paddler_model_card_fixture::smolvlm2_256m_mmproj::smolvlm2_256m_mmproj;

const ANSWER: &str = "two llamas";
const MAX_TOKENS: NonZeroU32 = NonZeroU32::new(16).unwrap();
const SLOTS: u16 = 2;
const TOKENS_THE_BATCH_HOLDS: u32 = 1;

#[test]
fn a_generation_step_overflowing_its_batch_fails_as_batch_assembly() {
    let runtime = LlamaRuntime::load(
        &smolvlm2_256m()
            .cached_path()
            .expect("the vision model must be in the Hugging Face cache"),
        agent_inference_runtime_context(SLOTS),
    )
    .expect("the vision model must load");
    let llama_context = runtime
        .create_text_generation_llama_context(u32::from(SLOTS))
        .expect("the context must be created");
    let sequence_context_size = llama_context.n_ctx_seq();
    let model = &runtime.loaded_llama_model.model;
    let request_preparer = FirstMessageContentRequestPreparer {
        llama_runtime: &runtime,
        sequence_context_size,
    }
    .into_multimodal_generation_request_preparer(
        smolvlm2_256m_mmproj()
            .cached_path()
            .expect("the multimodal projection must be in the Hugging Face cache"),
    )
    .expect(
        "the first message content chat template and the multimodal projection must be prepared",
    );
    let multimodal_ingestion = MultimodalIngestion::for_context(
        &llama_context,
        request_preparer.model_capabilities.clone(),
    );
    let image_data_uri = LLAMAS_JPEG.data_uri().expect("the test image must load");
    let image_question = || {
        ConversationGeneration::new(
            ContinueFromConversationHistoryParams {
                grammar: Some(gbnf_literal(ANSWER)),
                ..SingleImageQuestion {
                    image_data_uri: image_data_uri.clone(),
                    max_tokens: MAX_TOKENS,
                    question: "How many llamas are in the image?",
                }
                .into_conversation_params()
            },
            runtime
                .inference_runtime_context
                .slot_aggregated_status
                .clone(),
        )
    };
    let ConversationGeneration {
        agent_request: first_image_question,
        generate_tokens_stop_tx: _first_generate_tokens_stop_tx,
        generated_tokens_rx: first_generated_tokens_rx,
    } = image_question();
    let ConversationGeneration {
        agent_request: second_image_question,
        generate_tokens_stop_tx: _second_generate_tokens_stop_tx,
        generated_tokens_rx: second_generated_tokens_rx,
    } = image_question();
    let (scheduler_message_tx, scheduler_message_rx) = unbounded_channel();

    for agent_request in [first_image_question, second_image_question] {
        scheduler_message_tx
            .send(SchedulerMessage::Command(
                request_preparer
                    .prepare_conversation_history(agent_request)
                    .expect("the image question must be prepared"),
            ))
            .expect("the scheduler must accept the image question");
    }
    scheduler_message_tx
        .send(SchedulerMessage::Shutdown)
        .expect("the scheduler must accept the shutdown");

    ContinuousBatchScheduler::new(ContinuousBatchSchedulerParams {
        agent_shutdown: CancellationToken::new(),
        batch: LlamaBatch::new(ContextLimits {
            batch: TOKENS_THE_BATCH_HOLDS,
            ..llama_context.limits()
        }),
        llama_context,
        scheduler_context: ContinuousBatchSchedulerContext {
            agent_name: None,
            desired_slots_total: SLOTS,
            model,
            multimodal_ingestion,
            sequence_context_size,
        },
        scheduler_message_rx,
    })
    .run();

    assert!(matches!(
        [
            last_generated_token_result(first_generated_tokens_rx),
            last_generated_token_result(second_generated_tokens_rx),
        ],
        [
            Some(GeneratedTokenResult::BatchAssemblyFailed(_)),
            Some(GeneratedTokenResult::BatchAssemblyFailed(_)),
        ]
    ));
}
