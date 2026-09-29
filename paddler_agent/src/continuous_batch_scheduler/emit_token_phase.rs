use llama_cpp_bindings::SampledToken;
use llama_cpp_bindings::token_piece::TokenPiece;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use tokio::sync::mpsc;

use crate::continuous_batch_scheduler::classified_token::ClassifiedToken;
use crate::continuous_batch_scheduler::emit_token_outcome::EmitTokenOutcome;

const fn token_to_event(sampled_token: SampledToken, text: String) -> GeneratedTokenResult {
    match sampled_token {
        SampledToken::Content(_) => GeneratedTokenResult::ContentToken(text),
        SampledToken::Reasoning(_) => GeneratedTokenResult::ReasoningToken(text),
        SampledToken::ToolCall(_) => GeneratedTokenResult::ToolCallToken(text),
        SampledToken::Undeterminable(_) => GeneratedTokenResult::UndeterminableToken(text),
    }
}

#[must_use]
pub fn run(
    generated_tokens_tx: &mpsc::UnboundedSender<GeneratedTokenResult>,
    ClassifiedToken {
        sampled_token,
        piece,
        ..
    }: ClassifiedToken,
) -> EmitTokenOutcome {
    let TokenPiece::Visible(text) = piece else {
        return EmitTokenOutcome::Emitted;
    };

    if text.is_empty() {
        return EmitTokenOutcome::Emitted;
    }

    if generated_tokens_tx
        .send(token_to_event(sampled_token, text))
        .is_err()
    {
        return EmitTokenOutcome::ChannelDropped;
    }

    EmitTokenOutcome::Emitted
}

#[cfg(test)]
mod tests {
    use std::mem::discriminant;

    use llama_cpp_bindings::SampledToken;
    use llama_cpp_bindings::token::LlamaToken;
    use llama_cpp_bindings::token_piece::TokenPiece;
    use paddler_messaging::generated_token_result::GeneratedTokenResult;
    use tokio::sync::mpsc;
    use tokio::sync::mpsc::error::TryRecvError;

    use super::run;
    use crate::continuous_batch_scheduler::classified_token::ClassifiedToken;
    use crate::continuous_batch_scheduler::emit_token_outcome::EmitTokenOutcome;

    fn classified(sampled_token: SampledToken, piece: TokenPiece) -> ClassifiedToken {
        ClassifiedToken {
            sampled_token,
            piece,
            was_in_tool_call: false,
            is_in_tool_call: false,
        }
    }

    fn emitted(sampled_token: SampledToken, text: &str) -> GeneratedTokenResult {
        let (generated_tokens_tx, mut generated_tokens_rx) = mpsc::unbounded_channel();

        let outcome = run(
            &generated_tokens_tx,
            classified(sampled_token, TokenPiece::Visible(text.to_owned())),
        );

        assert_eq!(
            discriminant(&outcome),
            discriminant(&EmitTokenOutcome::Emitted)
        );

        generated_tokens_rx.try_recv().unwrap()
    }

    #[test]
    fn marker_piece_is_not_sent() {
        let (generated_tokens_tx, mut generated_tokens_rx) = mpsc::unbounded_channel();

        let outcome = run(
            &generated_tokens_tx,
            classified(
                SampledToken::Reasoning(LlamaToken::new(1)),
                TokenPiece::Marker("<think>".to_owned()),
            ),
        );

        assert_eq!(
            discriminant(&outcome),
            discriminant(&EmitTokenOutcome::Emitted)
        );
        assert_eq!(generated_tokens_rx.try_recv(), Err(TryRecvError::Empty));
    }

    #[test]
    fn empty_visible_piece_is_not_sent() {
        let (generated_tokens_tx, mut generated_tokens_rx) = mpsc::unbounded_channel();

        let outcome = run(
            &generated_tokens_tx,
            classified(
                SampledToken::Content(LlamaToken::new(2)),
                TokenPiece::Visible(String::new()),
            ),
        );

        assert_eq!(
            discriminant(&outcome),
            discriminant(&EmitTokenOutcome::Emitted)
        );
        assert_eq!(generated_tokens_rx.try_recv(), Err(TryRecvError::Empty));
    }

    #[test]
    fn content_token_emits_content_event() {
        assert_eq!(
            emitted(SampledToken::Content(LlamaToken::new(3)), "hi"),
            GeneratedTokenResult::ContentToken("hi".to_owned())
        );
    }

    #[test]
    fn reasoning_token_emits_reasoning_event() {
        assert_eq!(
            emitted(SampledToken::Reasoning(LlamaToken::new(4)), "think"),
            GeneratedTokenResult::ReasoningToken("think".to_owned())
        );
    }

    #[test]
    fn tool_call_token_emits_tool_call_event() {
        assert_eq!(
            emitted(SampledToken::ToolCall(LlamaToken::new(5)), "{"),
            GeneratedTokenResult::ToolCallToken("{".to_owned())
        );
    }

    #[test]
    fn undeterminable_token_emits_undeterminable_event() {
        assert_eq!(
            emitted(SampledToken::Undeterminable(LlamaToken::new(6)), "?"),
            GeneratedTokenResult::UndeterminableToken("?".to_owned())
        );
    }

    #[test]
    fn dropped_receiver_returns_channel_dropped() {
        let (generated_tokens_tx, generated_tokens_rx) = mpsc::unbounded_channel();

        drop(generated_tokens_rx);

        let outcome = run(
            &generated_tokens_tx,
            classified(
                SampledToken::Content(LlamaToken::new(7)),
                TokenPiece::Visible("hi".to_owned()),
            ),
        );

        assert_eq!(
            discriminant(&outcome),
            discriminant(&EmitTokenOutcome::ChannelDropped)
        );
    }
}
