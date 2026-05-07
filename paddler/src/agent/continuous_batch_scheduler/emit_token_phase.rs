use llama_cpp_bindings::SampledToken;
use paddler_types::generated_token_result::GeneratedTokenResult;
use tokio::sync::mpsc;

use crate::agent::continuous_batch_active_request::ContinuousBatchActiveRequest;
use crate::agent::continuous_batch_scheduler::classified_token::ClassifiedToken;
use crate::agent::continuous_batch_scheduler::emit_token_outcome::EmitTokenOutcome;

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
    use anyhow::Result;
    use anyhow::bail;
    use llama_cpp_bindings::SampledToken;
    use llama_cpp_bindings::token::LlamaToken;
    use paddler_types::generated_token_result::GeneratedTokenResult;
    use tokio::sync::mpsc;

    use super::emit_classified;
    use crate::agent::continuous_batch_scheduler::classified_token::ClassifiedToken;
    use crate::agent::continuous_batch_scheduler::emit_token_outcome::EmitTokenOutcome;

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
    fn empty_visible_piece_emits_empty_string_without_sending() -> Result<()> {
        let (tx, mut rx) = mpsc::unbounded_channel::<GeneratedTokenResult>();
        let classified = classified_with_piece(SampledToken::Content(LlamaToken::new(1)), "");

        match emit_classified(&classified, &tx) {
            EmitTokenOutcome::Emitted(piece) if piece.is_empty() => {}
            other => bail!("expected Emitted(\"\"), got {other:?}"),
        }

        match rx.try_recv() {
            Err(mpsc::error::TryRecvError::Empty) => Ok(()),
            other => bail!("expected empty channel, got {other:?}"),
        }
    }

    #[test]
    fn content_token_emits_content_event() -> Result<()> {
        let (tx, mut rx) = mpsc::unbounded_channel::<GeneratedTokenResult>();
        let classified = classified_with_piece(SampledToken::Content(LlamaToken::new(2)), "hi");

        emit_classified(&classified, &tx);

        match rx.try_recv() {
            Ok(GeneratedTokenResult::ContentToken(text)) if text == "hi" => Ok(()),
            other => bail!("expected ContentToken(\"hi\"), got {other:?}"),
        }
    }

    #[test]
    fn reasoning_token_emits_reasoning_event() -> Result<()> {
        let (tx, mut rx) = mpsc::unbounded_channel::<GeneratedTokenResult>();
        let classified =
            classified_with_piece(SampledToken::Reasoning(LlamaToken::new(3)), "think");

        emit_classified(&classified, &tx);

        match rx.try_recv() {
            Ok(GeneratedTokenResult::ReasoningToken(text)) if text == "think" => Ok(()),
            other => bail!("expected ReasoningToken(\"think\"), got {other:?}"),
        }
    }

    #[test]
    fn tool_call_token_emits_tool_call_event() -> Result<()> {
        let (tx, mut rx) = mpsc::unbounded_channel::<GeneratedTokenResult>();
        let classified = classified_with_piece(SampledToken::ToolCall(LlamaToken::new(4)), "{");

        emit_classified(&classified, &tx);

        match rx.try_recv() {
            Ok(GeneratedTokenResult::ToolCallToken(text)) if text == "{" => Ok(()),
            other => bail!("expected ToolCallToken(\"{{\"), got {other:?}"),
        }
    }

    #[test]
    fn undeterminable_token_emits_undeterminable_event() -> Result<()> {
        let (tx, mut rx) = mpsc::unbounded_channel::<GeneratedTokenResult>();
        let classified =
            classified_with_piece(SampledToken::Undeterminable(LlamaToken::new(5)), "?");

        emit_classified(&classified, &tx);

        match rx.try_recv() {
            Ok(GeneratedTokenResult::UndeterminableToken(text)) if text == "?" => Ok(()),
            other => bail!("expected UndeterminableToken(\"?\"), got {other:?}"),
        }
    }

    #[test]
    fn dropped_receiver_returns_channel_dropped() -> Result<()> {
        let (tx, rx) = mpsc::unbounded_channel::<GeneratedTokenResult>();
        drop(rx);
        let classified = classified_with_piece(SampledToken::Content(LlamaToken::new(6)), "hi");

        match emit_classified(&classified, &tx) {
            EmitTokenOutcome::ChannelDropped => Ok(()),
            EmitTokenOutcome::Emitted(piece) => {
                bail!("expected ChannelDropped on dropped receiver, got Emitted({piece:?})")
            }
        }
    }
}
