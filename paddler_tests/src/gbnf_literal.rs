use paddler_messaging::grammar_constraint::GrammarConstraint;

#[must_use]
pub fn gbnf_literal(text: &str) -> GrammarConstraint {
    let escaped_text = text.replace('\\', "\\\\").replace('"', "\\\"");

    GrammarConstraint::Gbnf {
        grammar: format!("root ::= \"{escaped_text}\""),
        root: "root".to_owned(),
    }
}
