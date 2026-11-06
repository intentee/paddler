#![cfg(feature = "tests_that_use_llms")]

use llama_cpp_bindings::error::parse_chat_message_error::ParseChatMessageError;
use llama_cpp_fault_injection::injected_fault_message::injected_fault_message;

use paddler_agent_text_generation::continuous_batch_terminal_outcome::ContinuousBatchTerminalOutcome;
use paddler_agent_text_generation::text_generation_error::TextGenerationError;

use crate::weather_answer_classification_failing_after_reads::WeatherAnswerClassificationFailingAfterReads;

const READS_BEFORE_THE_GENERATED_TOKEN: usize = 1;

#[test]
fn a_generated_token_llama_cpp_cannot_classify_ends_the_generation() {
    assert_eq!(
        WeatherAnswerClassificationFailingAfterReads {
            reads_that_succeed: READS_BEFORE_THE_GENERATED_TOKEN,
        }
        .single_token_generation_outcome(),
        ContinuousBatchTerminalOutcome::EmitToClient(
            TextGenerationError::AnswerClassificationFailed(ParseChatMessageError::Reported {
                message: injected_fault_message(),
            })
            .into_generated_token_result(None)
        )
    );
}
