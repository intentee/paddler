use paddler_messaging::grammar_constraint::GrammarConstraint;

use crate::constants::UNENDING_GENERATION_CYCLE;

#[must_use]
pub fn unending_grammar() -> GrammarConstraint {
    GrammarConstraint::Gbnf {
        grammar: format!("root ::= \"{UNENDING_GENERATION_CYCLE}\" root"),
        root: "root".to_owned(),
    }
}
