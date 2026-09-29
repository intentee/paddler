use paddler_messaging::grammar_constraint::GrammarConstraint;

use crate::generation_request_rejection::GenerationRequestRejection;
use crate::grammar_sampler::GrammarSampler;

pub fn resolve_grammar(
    grammar: Option<GrammarConstraint>,
) -> Result<Option<GrammarSampler>, GenerationRequestRejection> {
    grammar
        .map(GrammarSampler::new)
        .transpose()
        .map_err(GenerationRequestRejection::GrammarConversionFailed)
}

#[cfg(test)]
mod tests {
    use std::mem::Discriminant;
    use std::mem::discriminant;

    use llama_cpp_bindings::error::JsonSchemaToGrammarError;
    use paddler_messaging::grammar_constraint::GrammarConstraint;

    use super::resolve_grammar;
    use crate::generation_request_rejection::GenerationRequestRejection;
    use crate::grammar_sampler::GrammarSampler;

    fn yes_or_no_grammar() -> GrammarConstraint {
        GrammarConstraint::Gbnf {
            grammar: "root ::= \"yes\" | \"no\"".to_owned(),
            root: "root".to_owned(),
        }
    }

    fn resolved(
        grammar: Option<GrammarConstraint>,
    ) -> Result<Option<GrammarSampler>, Discriminant<GenerationRequestRejection>> {
        resolve_grammar(grammar).map_err(|rejection| discriminant(&rejection))
    }

    #[test]
    fn returns_none_when_grammar_is_absent() {
        assert_eq!(resolved(None), Ok(None));
    }

    #[test]
    fn returns_sampler_for_valid_grammar() {
        assert_eq!(
            resolved(Some(yes_or_no_grammar())),
            Ok(Some(GrammarSampler::new(yes_or_no_grammar()).unwrap()))
        );
    }

    #[test]
    fn rejects_json_schema_that_cannot_be_converted_to_grammar() {
        let grammar = GrammarConstraint::JsonSchema {
            schema: "not valid json at all".to_owned(),
        };

        assert_eq!(
            resolved(Some(grammar)),
            Err(discriminant(
                &GenerationRequestRejection::GrammarConversionFailed(
                    JsonSchemaToGrammarError::NotEnoughMemory
                )
            ))
        );
    }
}
