use paddler_messaging::grammar_constraint::GrammarConstraint;

use crate::generation_request_rejection::GenerationRequestRejection;

pub const fn require_grammar_compatible_with_thinking(
    grammar: Option<&GrammarConstraint>,
    enable_thinking: bool,
) -> Result<(), GenerationRequestRejection> {
    if grammar.is_some() && enable_thinking {
        Err(GenerationRequestRejection::GrammarIncompatibleWithThinking)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::mem::discriminant;

    use paddler_messaging::grammar_constraint::GrammarConstraint;

    use super::require_grammar_compatible_with_thinking;
    use crate::generation_request_rejection::GenerationRequestRejection;

    #[test]
    fn rejects_a_grammar_when_thinking_is_enabled() {
        let grammar = GrammarConstraint::Gbnf {
            grammar: "root ::= \"yes\" | \"no\"".to_owned(),
            root: "root".to_owned(),
        };

        assert_eq!(
            require_grammar_compatible_with_thinking(Some(&grammar), true)
                .map_err(|rejection| discriminant(&rejection)),
            Err(discriminant(
                &GenerationRequestRejection::GrammarIncompatibleWithThinking
            ))
        );
    }

    #[test]
    fn accepts_thinking_without_a_grammar() {
        assert!(require_grammar_compatible_with_thinking(None, true).is_ok());
    }
}
