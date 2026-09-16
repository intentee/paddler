use anyhow::Result;
use llama_cpp_bindings::sampling::LlamaSampler;
use log::error;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use tokio::sync::mpsc;

use crate::send_generated_token_result_or_warn::send_generated_token_result_or_warn;

pub fn resolve(
    agent_name: Option<&str>,
    result: Result<LlamaSampler>,
    generated_tokens_tx: &mpsc::UnboundedSender<GeneratedTokenResult>,
) -> Result<LlamaSampler> {
    match result {
        Ok(sampler_chain) => Ok(sampler_chain),
        Err(err) => {
            let message = format!("{agent_name:?}: failed to initialize sampler chain: {err:#}");

            error!("{message}");
            send_generated_token_result_or_warn(
                agent_name,
                generated_tokens_tx,
                GeneratedTokenResult::SamplerError(message),
            );

            Err(err)
        }
    }
}

#[cfg(test)]
mod tests {
    use anyhow::anyhow;
    use llama_cpp_bindings::sampling::LlamaSampler;
    use paddler_messaging::generated_token_result::GeneratedTokenResult;
    use tokio::sync::mpsc;

    use super::resolve;

    #[test]
    fn successful_sampler_chain_initialization_returns_the_chain_without_sending_an_error() {
        let (generated_tokens_tx, mut generated_tokens_rx) = mpsc::unbounded_channel();
        let sampler_chain = LlamaSampler::greedy().unwrap();

        let chain = resolve(Some("agent-1"), Ok(sampler_chain), &generated_tokens_tx);

        assert!(chain.is_ok());
        assert!(generated_tokens_rx.try_recv().is_err());
    }

    #[test]
    fn failed_sampler_chain_initialization_sends_the_full_error_and_returns_the_error() {
        let (generated_tokens_tx, mut generated_tokens_rx) = mpsc::unbounded_channel();

        let chain = resolve(
            Some("agent-1"),
            Err(anyhow!("distribution sampler allocation failed")),
            &generated_tokens_tx,
        );

        assert_eq!(
            chain.err().unwrap().to_string(),
            "distribution sampler allocation failed"
        );
        assert!(matches!(
            generated_tokens_rx.try_recv(),
            Ok(GeneratedTokenResult::SamplerError(message))
                if message == "Some(\"agent-1\"): failed to initialize sampler chain: distribution sampler allocation failed"
        ));
    }
}
