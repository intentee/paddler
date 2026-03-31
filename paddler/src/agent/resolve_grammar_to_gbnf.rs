use anyhow::Result;
use anyhow::anyhow;
use llama_cpp_bindings::json_schema_to_grammar;
use paddler_types::grammar_constraint::GrammarConstraint;

use crate::agent::resolved_grammar::ResolvedGrammar;

pub fn resolve_grammar_to_gbnf(grammar_constraint: &GrammarConstraint) -> Result<ResolvedGrammar> {
    match grammar_constraint {
        GrammarConstraint::Gbnf { grammar, root } => Ok(ResolvedGrammar {
            grammar_string: grammar.clone(),
            root_rule: root.clone(),
        }),
        GrammarConstraint::JsonSchema { schema } => {
            let grammar_string = json_schema_to_grammar(schema)
                .map_err(|err| anyhow!("Failed to convert JSON schema to grammar: {err}"))?;

            Ok(ResolvedGrammar {
                grammar_string,
                root_rule: "root".to_owned(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use super::*;

    #[test]
    fn resolves_gbnf_variant() -> Result<()> {
        let constraint = GrammarConstraint::Gbnf {
            grammar: "root ::= \"yes\" | \"no\"".to_owned(),
            root: "root".to_owned(),
        };

        let resolved = resolve_grammar_to_gbnf(&constraint)?;

        assert_eq!(resolved.grammar_string, "root ::= \"yes\" | \"no\"");
        assert_eq!(resolved.root_rule, "root");

        Ok(())
    }

    #[test]
    fn resolves_json_schema_variant() -> Result<()> {
        let constraint = GrammarConstraint::JsonSchema {
            schema: r#"{"type": "object", "properties": {"name": {"type": "string"}}}"#.to_owned(),
        };

        let resolved = resolve_grammar_to_gbnf(&constraint)?;

        assert!(!resolved.grammar_string.is_empty());
        assert_eq!(resolved.root_rule, "root");

        Ok(())
    }

    #[test]
    fn returns_error_for_invalid_json_schema() {
        let constraint = GrammarConstraint::JsonSchema {
            schema: "not valid json".to_owned(),
        };

        let result = resolve_grammar_to_gbnf(&constraint);

        assert!(result.is_err());
    }
}
