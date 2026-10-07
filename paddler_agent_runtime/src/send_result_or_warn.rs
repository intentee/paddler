use log::warn;
use tokio::sync::mpsc;

pub fn send_result_or_warn<TResult>(
    agent_name: Option<&str>,
    result_tx: &mpsc::UnboundedSender<TResult>,
    result: TResult,
) {
    if result_tx.send(result).is_err() {
        warn!("{agent_name:?}: failed to send result to client (receiver dropped)");
    }
}

#[cfg(test)]
mod tests {
    use log::LevelFilter;
    use log::set_max_level;
    use tokio::sync::mpsc;

    use paddler_messaging::generated_token_result::GeneratedTokenResult;

    use super::send_result_or_warn;

    #[test]
    fn delivers_result_to_a_live_receiver() {
        let (generated_tokens_tx, mut generated_tokens_rx) = mpsc::unbounded_channel();

        send_result_or_warn(
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
        set_max_level(LevelFilter::Trace);

        let (generated_tokens_tx, generated_tokens_rx) = mpsc::unbounded_channel();

        drop(generated_tokens_rx);

        send_result_or_warn(
            None,
            &generated_tokens_tx,
            GeneratedTokenResult::SamplerError("boom".to_owned()),
        );

        assert!(generated_tokens_tx.is_closed());
    }
}
