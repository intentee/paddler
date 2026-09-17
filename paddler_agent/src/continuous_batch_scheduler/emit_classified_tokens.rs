use paddler_messaging::generated_token_result::GeneratedTokenResult;
use tokio::sync::mpsc;

use crate::continuous_batch_scheduler::classified_token::ClassifiedToken;
use crate::continuous_batch_scheduler::client_stream_status::ClientStreamStatus;
use crate::continuous_batch_scheduler::emit_token_outcome::EmitTokenOutcome;
use crate::continuous_batch_scheduler::emit_token_phase;
use crate::continuous_batch_scheduler::tool_call_pass;
use crate::tool_call_pipeline::ToolCallPipeline;

#[must_use]
pub fn run(
    mut tool_call_pipeline: Option<&mut ToolCallPipeline>,
    generated_tokens_tx: &mpsc::UnboundedSender<GeneratedTokenResult>,
    classified_tokens: &[ClassifiedToken],
) -> ClientStreamStatus {
    for classified in classified_tokens {
        match emit_token_phase::run(classified, generated_tokens_tx) {
            EmitTokenOutcome::Emitted(_) => {}
            EmitTokenOutcome::ChannelDropped => return ClientStreamStatus::Dropped,
        }

        if let Some(event) = tool_call_pass::run(tool_call_pipeline.as_deref_mut(), classified)
            && generated_tokens_tx.send(event).is_err()
        {
            return ClientStreamStatus::Dropped;
        }
    }

    ClientStreamStatus::Open
}

#[cfg(test)]
mod tests {
    use llama_cpp_bindings::SampledToken;
    use llama_cpp_bindings::token::LlamaToken;
    use paddler_messaging::generated_token_result::GeneratedTokenResult;
    use tokio::sync::mpsc;

    use super::run;
    use crate::continuous_batch_scheduler::classified_token::ClassifiedToken;
    use crate::continuous_batch_scheduler::client_stream_status::ClientStreamStatus;

    fn content_token(piece: &str) -> ClassifiedToken {
        ClassifiedToken {
            sampled_token: SampledToken::Content(LlamaToken::new(1)),
            was_in_tool_call: false,
            is_in_tool_call: false,
            visible_piece: piece.to_owned(),
            raw_piece: piece.to_owned(),
        }
    }

    #[test]
    fn every_token_reaches_a_listening_client() {
        let (generated_tokens_tx, mut generated_tokens_rx) =
            mpsc::unbounded_channel::<GeneratedTokenResult>();

        let status = run(
            None,
            &generated_tokens_tx,
            &[content_token("he"), content_token("llo")],
        );

        assert_eq!(status, ClientStreamStatus::Open);
        assert_eq!(
            generated_tokens_rx
                .try_recv()
                .unwrap()
                .token_text()
                .unwrap(),
            "he"
        );
        assert_eq!(
            generated_tokens_rx
                .try_recv()
                .unwrap()
                .token_text()
                .unwrap(),
            "llo"
        );
    }

    #[test]
    fn emitting_to_a_disconnected_client_reports_the_stream_as_dropped() {
        let (generated_tokens_tx, generated_tokens_rx) =
            mpsc::unbounded_channel::<GeneratedTokenResult>();

        drop(generated_tokens_rx);

        assert_eq!(
            run(None, &generated_tokens_tx, &[content_token("hello")]),
            ClientStreamStatus::Dropped
        );
    }

    #[test]
    fn an_empty_batch_leaves_the_stream_open_without_emitting() {
        let (generated_tokens_tx, mut generated_tokens_rx) =
            mpsc::unbounded_channel::<GeneratedTokenResult>();

        assert_eq!(
            run(None, &generated_tokens_tx, &[]),
            ClientStreamStatus::Open
        );
        assert!(generated_tokens_rx.try_recv().is_err());
    }
}
