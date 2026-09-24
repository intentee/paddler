use paddler_messaging::grammar_constraint::GrammarConstraint;

use crate::generation_request_rejection::GenerationRequestRejection;
use crate::grammar_sampler::GrammarSampler;

pub fn resolve_grammar(
    grammar: Option<&GrammarConstraint>,
    enable_thinking: bool,
) -> Result<Option<GrammarSampler>, GenerationRequestRejection> {
    let Some(grammar_constraint) = grammar else {
        return Ok(None);
    };

    if enable_thinking {
        return Err(GenerationRequestRejection::GrammarIncompatibleWithThinking);
    }

    GrammarSampler::new(grammar_constraint)
        .map(Some)
        .map_err(GenerationRequestRejection::GrammarConversionFailed)
}

#[cfg(test)]
mod tests {
    use paddler_messaging::grammar_constraint::GrammarConstraint;

    use super::resolve_grammar;
    use crate::generation_request_rejection::GenerationRequestRejection;

    fn yes_or_no_grammar() -> GrammarConstraint {
        GrammarConstraint::Gbnf {
            grammar: "root ::= \"yes\" | \"no\"".to_owned(),
            root: "root".to_owned(),
        }
    }

    #[test]
    fn returns_none_when_grammar_is_absent() {
        assert!(matches!(resolve_grammar(None, false), Ok(None)));
    }

    #[test]
    fn rejects_grammar_when_thinking_is_enabled() {
        assert!(matches!(
            resolve_grammar(Some(&yes_or_no_grammar()), true),
            Err(GenerationRequestRejection::GrammarIncompatibleWithThinking)
        ));
    }

    #[test]
    fn returns_sampler_for_valid_grammar() {
        assert!(matches!(
            resolve_grammar(Some(&yes_or_no_grammar()), false),
            Ok(Some(_))
        ));
    }

    #[test]
    fn rejects_json_schema_that_cannot_be_converted_to_grammar() {
        let grammar = GrammarConstraint::JsonSchema {
            schema: "not valid json at all".to_owned(),
        };

        assert!(matches!(
            resolve_grammar(Some(&grammar), false),
            Err(GenerationRequestRejection::GrammarConversionFailed(_))
        ));
    }
}
