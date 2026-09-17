use anyhow::Result;
use anyhow::anyhow;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::grammar_constraint::GrammarConstraint;
use tokio::sync::mpsc;

use crate::grammar_engagement::GrammarEngagement;
use crate::grammar_sampler::GrammarSampler;
use crate::model_constants::ModelConstants;

pub fn resolve_grammar(
    grammar: Option<&GrammarConstraint>,
    enable_thinking: bool,
    model_constants: &ModelConstants,
    generated_tokens_tx: &mpsc::UnboundedSender<GeneratedTokenResult>,
) -> Result<Option<GrammarSampler>> {
    let Some(grammar_constraint) = grammar else {
        return Ok(None);
    };

    let engagement = GrammarEngagement::for_thinking(enable_thinking);

    if engagement == GrammarEngagement::AfterReasoning && !model_constants.closes_reasoning {
        let message = "Grammar constraints require thinking mode to end with a reasoning-close marker, which this model does not expose".to_owned();

        generated_tokens_tx
            .send(GeneratedTokenResult::GrammarRequiresReasoningCloseMarker(
                message.clone(),
            ))
            .map_err(|err| anyhow!("Failed to send missing reasoning-close marker error: {err}"))?;

        return Err(anyhow!(message));
    }

    match GrammarSampler::new(grammar_constraint, engagement) {
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
    use llama_cpp_bindings::MarkerRole;
    use llama_cpp_bindings::MarkerRoleCandidate;
    use llama_cpp_bindings::StreamingMarkers;
    use llama_cpp_bindings::token::LlamaToken;

    use super::*;

    fn model_constants(roles: Vec<MarkerRole>) -> ModelConstants {
        let candidates = roles
            .into_iter()
            .enumerate()
            .map(|(index, role)| MarkerRoleCandidate {
                tokens: vec![LlamaToken::new(i32::try_from(index).unwrap() + 1)],
                role,
            })
            .collect::<Vec<_>>();

        let streaming_markers = StreamingMarkers::from_candidates(candidates).unwrap();

        let closes_reasoning = streaming_markers
            .iter()
            .any(|marker| marker.roles().contains(&MarkerRole::ReasoningClose));

        ModelConstants {
            closes_reasoning,
            n_vocab: 32,
            streaming_markers,
            token_bos_str: String::new(),
            token_eos_str: String::new(),
            token_nl_str: String::new(),
        }
    }

    fn reasoning_model() -> ModelConstants {
        model_constants(vec![MarkerRole::ReasoningOpen, MarkerRole::ReasoningClose])
    }

    fn markerless_model() -> ModelConstants {
        model_constants(Vec::new())
    }

    fn gbnf_grammar() -> GrammarConstraint {
        GrammarConstraint::Gbnf {
            grammar: "root ::= \"yes\" | \"no\"".to_owned(),
            root: "root".to_owned(),
        }
    }

    #[test]
    fn returns_none_when_grammar_is_absent() {
        let (generated_tokens_tx, mut generated_tokens_rx) = mpsc::unbounded_channel();

        let resolved =
            resolve_grammar(None, false, &reasoning_model(), &generated_tokens_tx).unwrap();

        assert!(resolved.is_none());
        assert!(generated_tokens_rx.try_recv().is_err());
    }

    #[test]
    fn returns_sampler_for_valid_grammar() {
        let (generated_tokens_tx, mut generated_tokens_rx) = mpsc::unbounded_channel();

        let resolved = resolve_grammar(
            Some(&gbnf_grammar()),
            false,
            &reasoning_model(),
            &generated_tokens_tx,
        )
        .unwrap();

        assert!(resolved.is_some());
        assert!(generated_tokens_rx.try_recv().is_err());
    }

    #[test]
    fn thinking_is_allowed_when_the_model_closes_reasoning() {
        let (generated_tokens_tx, mut generated_tokens_rx) = mpsc::unbounded_channel();

        let resolved = resolve_grammar(
            Some(&gbnf_grammar()),
            true,
            &reasoning_model(),
            &generated_tokens_tx,
        )
        .unwrap();

        assert!(resolved.is_some());
        assert!(generated_tokens_rx.try_recv().is_err());
    }

    #[test]
    fn thinking_is_rejected_when_the_model_never_closes_reasoning() {
        let (generated_tokens_tx, mut generated_tokens_rx) = mpsc::unbounded_channel();

        let result = resolve_grammar(
            Some(&gbnf_grammar()),
            true,
            &markerless_model(),
            &generated_tokens_tx,
        );

        assert!(result.is_err());

        let event = generated_tokens_rx.try_recv().unwrap();

        assert!(matches!(
            event,
            GeneratedTokenResult::GrammarRequiresReasoningCloseMarker(_)
        ));
    }

    #[test]
    fn errors_when_the_missing_reasoning_close_marker_event_cannot_be_sent() {
        let (generated_tokens_tx, generated_tokens_rx) = mpsc::unbounded_channel();

        drop(generated_tokens_rx);

        let result = resolve_grammar(
            Some(&gbnf_grammar()),
            true,
            &markerless_model(),
            &generated_tokens_tx,
        );

        assert_eq!(
            result.err().unwrap().to_string(),
            "Failed to send missing reasoning-close marker error: channel closed"
        );
    }

    #[test]
    fn emits_syntax_error_event_and_errors_for_invalid_grammar() {
        let (generated_tokens_tx, mut generated_tokens_rx) = mpsc::unbounded_channel();
        let grammar = GrammarConstraint::JsonSchema {
            schema: "not valid json at all".to_owned(),
        };

        let result = resolve_grammar(
            Some(&grammar),
            false,
            &reasoning_model(),
            &generated_tokens_tx,
        );

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

        let result = resolve_grammar(
            Some(&grammar),
            false,
            &reasoning_model(),
            &generated_tokens_tx,
        );

        assert_eq!(
            result.err().unwrap().to_string(),
            "Failed to send grammar syntax error: channel closed"
        );
    }
}
