use llama_cpp_bindings::SampledToken;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use tokio::sync::mpsc;

use crate::continuous_batch_active_request::ContinuousBatchActiveRequest;
use crate::continuous_batch_scheduler::classified_token::ClassifiedToken;
use crate::continuous_batch_scheduler::emit_token_outcome::EmitTokenOutcome;

pub fn run(
    request: &mut ContinuousBatchActiveRequest,
    classified: &ClassifiedToken,
) -> EmitTokenOutcome {
    emit_classified(classified, &request.generated_tokens_tx)
}

fn emit_classified(
    classified: &ClassifiedToken,
    tx: &mpsc::UnboundedSender<GeneratedTokenResult>,
) -> EmitTokenOutcome {
    if classified.visible_piece.is_empty() {
        return EmitTokenOutcome::Emitted(String::new());
    }

    let piece = classified.visible_piece.clone();
    let event = token_to_event(classified.sampled_token, piece.clone());

    if tx.send(event).is_err() {
        return EmitTokenOutcome::ChannelDropped;
    }

    EmitTokenOutcome::Emitted(piece)
}

const fn token_to_event(sampled_token: SampledToken, piece: String) -> GeneratedTokenResult {
    match sampled_token {
        SampledToken::Content(_) => GeneratedTokenResult::ContentToken(piece),
        SampledToken::Reasoning(_) => GeneratedTokenResult::ReasoningToken(piece),
        SampledToken::ToolCall(_) => GeneratedTokenResult::ToolCallToken(piece),
        SampledToken::Undeterminable(_) => GeneratedTokenResult::UndeterminableToken(piece),
    }
}

#[cfg(test)]
mod tests {
    use std::mem::discriminant;

    use llama_cpp_bindings::SampledToken;
    use llama_cpp_bindings::token::LlamaToken;
    use tokio::sync::mpsc;

    use super::emit_classified;
    use crate::continuous_batch_scheduler::classified_token::ClassifiedToken;
    use crate::continuous_batch_scheduler::emit_token_outcome::EmitTokenOutcome;
    use paddler_messaging::generated_token_result::GeneratedTokenResult;

    fn classified_with_piece(sampled: SampledToken, piece: &str) -> ClassifiedToken {
        ClassifiedToken {
            sampled_token: sampled,
            was_in_tool_call: false,
            is_in_tool_call: false,
            visible_piece: piece.to_owned(),
            raw_piece: piece.to_owned(),
        }
    }

    #[test]
    fn empty_visible_piece_emits_empty_string_without_sending() {
        let (tx, mut rx) = mpsc::unbounded_channel::<GeneratedTokenResult>();
        let classified = classified_with_piece(SampledToken::Content(LlamaToken::new(1)), "");

        let outcome = emit_classified(&classified, &tx);

        assert_eq!(
            discriminant(&outcome),
            discriminant(&EmitTokenOutcome::Emitted(String::new())),
        );

        let receive_error = rx.try_recv().err().unwrap();

        assert_eq!(
            discriminant(&receive_error),
            discriminant(&mpsc::error::TryRecvError::Empty),
        );
    }

    #[test]
    fn content_token_emits_content_event() {
        let (tx, mut rx) = mpsc::unbounded_channel::<GeneratedTokenResult>();
        let classified = classified_with_piece(SampledToken::Content(LlamaToken::new(2)), "hi");

        let outcome = emit_classified(&classified, &tx);

        assert_eq!(
            discriminant(&outcome),
            discriminant(&EmitTokenOutcome::Emitted(String::new())),
        );

        let event = rx.try_recv().unwrap();

        assert_eq!(
            discriminant(&event),
            discriminant(&GeneratedTokenResult::ContentToken(String::new())),
        );
        assert_eq!(event.token_text().unwrap(), "hi");
    }

    #[test]
    fn reasoning_token_emits_reasoning_event() {
        let (tx, mut rx) = mpsc::unbounded_channel::<GeneratedTokenResult>();
        let classified =
            classified_with_piece(SampledToken::Reasoning(LlamaToken::new(3)), "think");

        emit_classified(&classified, &tx);

        let event = rx.try_recv().unwrap();

        assert_eq!(
            discriminant(&event),
            discriminant(&GeneratedTokenResult::ReasoningToken(String::new())),
        );
        assert_eq!(event.token_text().unwrap(), "think");
    }

    #[test]
    fn tool_call_token_emits_tool_call_event() {
        let (tx, mut rx) = mpsc::unbounded_channel::<GeneratedTokenResult>();
        let classified = classified_with_piece(SampledToken::ToolCall(LlamaToken::new(4)), "{");

        emit_classified(&classified, &tx);

        let event = rx.try_recv().unwrap();

        assert_eq!(
            discriminant(&event),
            discriminant(&GeneratedTokenResult::ToolCallToken(String::new())),
        );
        assert_eq!(event.token_text().unwrap(), "{");
    }

    #[test]
    fn undeterminable_token_emits_undeterminable_event() {
        let (tx, mut rx) = mpsc::unbounded_channel::<GeneratedTokenResult>();
        let classified =
            classified_with_piece(SampledToken::Undeterminable(LlamaToken::new(5)), "?");

        emit_classified(&classified, &tx);

        let event = rx.try_recv().unwrap();

        assert_eq!(
            discriminant(&event),
            discriminant(&GeneratedTokenResult::UndeterminableToken(String::new())),
        );
        assert_eq!(event.token_text().unwrap(), "?");
    }

    #[test]
    fn dropped_receiver_returns_channel_dropped() {
        let (tx, rx) = mpsc::unbounded_channel::<GeneratedTokenResult>();
        drop(rx);
        let classified = classified_with_piece(SampledToken::Content(LlamaToken::new(6)), "hi");

        let outcome = emit_classified(&classified, &tx);

        assert_eq!(
            discriminant(&outcome),
            discriminant(&EmitTokenOutcome::ChannelDropped),
        );
    }
}
