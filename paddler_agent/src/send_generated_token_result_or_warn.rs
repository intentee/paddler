use log::warn;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use tokio::sync::mpsc;

pub fn send_generated_token_result_or_warn(
    agent_name: Option<&str>,
    generated_tokens_tx: &mpsc::UnboundedSender<GeneratedTokenResult>,
    result: GeneratedTokenResult,
) {
    if generated_tokens_tx.send(result).is_err() {
        warn!("{agent_name:?}: failed to send result to client (receiver dropped)");
    }
}

#[cfg(test)]
mod tests {
    use log::LevelFilter;
    use paddler_messaging::generated_token_result::GeneratedTokenResult;
    use tokio::sync::mpsc;

    use super::send_generated_token_result_or_warn;

    #[test]
    fn delivers_result_to_a_live_receiver() {
        let (generated_tokens_tx, mut generated_tokens_rx) = mpsc::unbounded_channel();

        send_generated_token_result_or_warn(
            Some("agent"),
            &generated_tokens_tx,
            GeneratedTokenResult::SamplerError("boom".to_owned()),
        );

        assert!(matches!(
            generated_tokens_rx.try_recv(),
            Ok(GeneratedTokenResult::SamplerError(message)) if message == "boom"
        ));
    }

    #[test]
    fn warns_without_panicking_when_the_receiver_was_dropped() {
        log::set_max_level(LevelFilter::Trace);

        let (generated_tokens_tx, generated_tokens_rx) = mpsc::unbounded_channel();

        drop(generated_tokens_rx);

        send_generated_token_result_or_warn(
            None,
            &generated_tokens_tx,
            GeneratedTokenResult::SamplerError("boom".to_owned()),
        );

        assert!(generated_tokens_tx.is_closed());
    }
}
