use crate::generated_token_result::GeneratedTokenResult;
use crate::grammar_constraint::GrammarConstraint;
use anyhow::Result;
use anyhow::anyhow;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_none_when_grammar_is_absent() {
        let (generated_tokens_tx, mut generated_tokens_rx) = mpsc::unbounded_channel();

        let resolved = resolve_grammar(None, false, &generated_tokens_tx).unwrap();

        assert!(resolved.is_none());
        assert!(generated_tokens_rx.try_recv().is_err());
    }

    #[test]
    fn emits_incompatibility_event_and_errors_when_thinking_is_enabled() {
        let (generated_tokens_tx, mut generated_tokens_rx) = mpsc::unbounded_channel();
        let grammar = GrammarConstraint::Gbnf {
            grammar: "root ::= \"yes\" | \"no\"".to_owned(),
            root: "root".to_owned(),
        };

        let result = resolve_grammar(Some(&grammar), true, &generated_tokens_tx);

        assert!(result.is_err());

        let event = generated_tokens_rx.try_recv().unwrap();

        assert!(
            matches!(event, GeneratedTokenResult::GrammarIncompatibleWithThinking(message) if message == "Grammar constraints are incompatible with thinking mode")
        );
    }

    #[test]
    fn errors_when_incompatibility_event_cannot_be_sent() {
        let (generated_tokens_tx, generated_tokens_rx) = mpsc::unbounded_channel();

        drop(generated_tokens_rx);

        let grammar = GrammarConstraint::Gbnf {
            grammar: "root ::= \"yes\" | \"no\"".to_owned(),
            root: "root".to_owned(),
        };

        let result = resolve_grammar(Some(&grammar), true, &generated_tokens_tx);

        assert_eq!(
            result.err().unwrap().to_string(),
            "Failed to send grammar incompatibility error: channel closed"
        );
    }

    #[test]
    fn returns_sampler_for_valid_grammar() {
        let (generated_tokens_tx, mut generated_tokens_rx) = mpsc::unbounded_channel();
        let grammar = GrammarConstraint::Gbnf {
            grammar: "root ::= \"yes\" | \"no\"".to_owned(),
            root: "root".to_owned(),
        };

        let resolved = resolve_grammar(Some(&grammar), false, &generated_tokens_tx).unwrap();

        assert!(resolved.is_some());
        assert!(generated_tokens_rx.try_recv().is_err());
    }

    #[test]
    fn emits_syntax_error_event_and_errors_for_invalid_grammar() {
        let (generated_tokens_tx, mut generated_tokens_rx) = mpsc::unbounded_channel();
        let grammar = GrammarConstraint::JsonSchema {
            schema: "not valid json at all".to_owned(),
        };

        let result = resolve_grammar(Some(&grammar), false, &generated_tokens_tx);

        assert!(result.is_err());
        assert!(
            result
                .err()
                .unwrap()
                .to_string()
                .starts_with("Failed to create grammar sampler:")
        );

        let event = generated_tokens_rx.try_recv().unwrap();

        assert!(
            matches!(event, GeneratedTokenResult::GrammarSyntaxError(message) if message.starts_with("Failed to create grammar sampler:"))
        );
    }

    #[test]
    fn errors_when_syntax_error_event_cannot_be_sent() {
        let (generated_tokens_tx, generated_tokens_rx) = mpsc::unbounded_channel();

        drop(generated_tokens_rx);

        let grammar = GrammarConstraint::JsonSchema {
            schema: "not valid json at all".to_owned(),
        };

        let result = resolve_grammar(Some(&grammar), false, &generated_tokens_tx);

        assert_eq!(
            result.err().unwrap().to_string(),
            "Failed to send grammar syntax error: channel closed"
        );
    }
}
