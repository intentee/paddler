use anyhow::Result;
use anyhow::anyhow;
use paddler_types::generated_token_result::GeneratedTokenResult;
use paddler_types::grammar_constraint::GrammarConstraint;
use tokio::sync::mpsc;

use crate::agent::grammar_sampler::GrammarSampler;

pub fn resolve_grammar(
    grammar: Option<&GrammarConstraint>,
    enable_thinking: bool,
    generated_tokens_tx: &mpsc::UnboundedSender<GeneratedTokenResult>,
) -> Result<Option<GrammarSampler>> {
    let Some(grammar_constraint) = grammar else {
        return Ok(None);
    };

    if enable_thinking {
        let message = "Grammar constraints are incompatible with thinking mode".to_owned();

        generated_tokens_tx
            .send(GeneratedTokenResult::GrammarIncompatibleWithThinking(
                message.clone(),
            ))
            .map_err(|err| anyhow!("Failed to send grammar incompatibility error: {err}"))?;

        return Err(anyhow!(message));
    }

    match GrammarSampler::new(grammar_constraint) {
        Ok(sampler) => Ok(Some(sampler)),
        Err(err) => {
            let message = format!("Failed to create grammar sampler: {err}");

            generated_tokens_tx
                .send(GeneratedTokenResult::GrammarSyntaxError(message.clone()))
                .map_err(|send_err| anyhow!("Failed to send grammar syntax error: {send_err}"))?;

            Err(anyhow!(message))
        }
    }
}
