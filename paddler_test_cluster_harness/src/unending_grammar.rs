use paddler_messaging::grammar_constraint::GrammarConstraint;

pub const UNENDING_GENERATION_CYCLE: &str = "apple banana cherry ";

#[must_use]
pub fn unending_grammar() -> GrammarConstraint {
    GrammarConstraint::Gbnf {
        grammar: format!("root ::= \"{UNENDING_GENERATION_CYCLE}\" root"),
        root: "root".to_owned(),
    }
}
