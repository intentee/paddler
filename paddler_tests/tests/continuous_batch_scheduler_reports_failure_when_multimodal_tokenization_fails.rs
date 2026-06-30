#![cfg(feature = "tests_that_use_llms")]

use std::sync::Arc;
use std::sync::mpsc::channel;

use anyhow::Result;
use anyhow::anyhow;
use log::LevelFilter;
use paddler_agent::continue_from_conversation_history_request::ContinueFromConversationHistoryRequest;
use paddler_agent::continuous_batch_scheduler::ContinuousBatchScheduler;
use paddler_agent::continuous_batch_scheduler_command::ContinuousBatchSchedulerCommand;
use paddler_agent::slot_aggregated_status::SlotAggregatedStatus;
use paddler_agent::slot_guard::SlotGuard;
use paddler_messaging::conversation_history::ConversationHistory;
use paddler_messaging::conversation_message::ConversationMessage;
use paddler_messaging::conversation_message_content::ConversationMessageContent;
use paddler_messaging::conversation_message_content_part::ConversationMessageContentPart;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::image_url::ImageUrl;
use paddler_messaging::inference_parameters::InferenceParameters;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_tests::load_test_image_data_uri::load_test_image_data_uri;
use paddler_tests::loaded_test_model::LoadedTestModel;
use tokio::sync::mpsc;

#[test]
fn continuous_batch_scheduler_reports_failure_when_multimodal_tokenization_fails() -> Result<()> {
    log::set_max_level(LevelFilter::Trace);

    let loaded = LoadedTestModel::smolvlm2()?;
    let scheduler_context = loaded.multimodal_scheduler_context(InferenceParameters::default())?;
    let llama_context = loaded.new_context()?;

    let (command_tx, command_rx) = channel::<ContinuousBatchSchedulerCommand>();
    let mut scheduler =
        ContinuousBatchScheduler::new(command_rx, scheduler_context, llama_context, 1);

    let (generated_tokens_tx, mut generated_tokens_rx) = mpsc::unbounded_channel();
    let (_generate_tokens_stop_tx, generate_tokens_stop_rx) = mpsc::unbounded_channel();
    command_tx
        .send(
            ContinuousBatchSchedulerCommand::ContinueFromConversationHistory(
                ContinueFromConversationHistoryRequest {
                    generate_tokens_stop_rx,
                    generated_tokens_tx,
                    params: ContinueFromConversationHistoryParams {
                        add_generation_prompt: true,
                        conversation_history: ConversationHistory::new(vec![ConversationMessage {
                            content: ConversationMessageContent::Parts(vec![
                                ConversationMessageContentPart::ImageUrl {
                                    image_url: ImageUrl {
                                        url: load_test_image_data_uri()?,
                                    },
                                },
                                ConversationMessageContentPart::Text {
                                    text: "Describe this image.".to_owned(),
                                },
                            ]),
                            role: "user".to_owned(),
                        }]),
                        enable_thinking: false,
                        grammar: None,
                        max_tokens: 16,
                        parse_tool_calls: false,
                        tools: Vec::new(),
                    },
                    slot_guard: SlotGuard::new(Arc::new(SlotAggregatedStatus::new(1))),
                },
            ),
        )
        .map_err(|send_error| anyhow!("failed to queue multimodal request: {send_error}"))?;

    drop(command_tx);

    scheduler.run();

    assert!(matches!(
        generated_tokens_rx.try_recv(),
        Ok(GeneratedTokenResult::SamplerError(_))
    ));

    Ok(())
}
