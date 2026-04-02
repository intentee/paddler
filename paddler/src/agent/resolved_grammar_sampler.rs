use anyhow::Result;
use anyhow::anyhow;
use llama_cpp_bindings::json_schema_to_grammar;
use llama_cpp_bindings::model::LlamaModel;
use llama_cpp_bindings::sampling::LlamaSampler;
use paddler_types::grammar_constraint::GrammarConstraint;

pub struct ResolvedGrammarSampler {
    grammar_string: String,
    root_rule: String,
}

impl ResolvedGrammarSampler {
    pub fn new(grammar_constraint: &GrammarConstraint) -> Result<Self> {
        let (grammar_string, root_rule) = match grammar_constraint {
            GrammarConstraint::Gbnf { grammar, root } => (grammar.clone(), root.clone()),
            GrammarConstraint::JsonSchema { schema } => {
                let grammar_string = json_schema_to_grammar(schema)
                    .map_err(|err| anyhow!("Failed to convert JSON schema to grammar: {err}"))?;

                (grammar_string, "root".to_owned())
            }
        };

        Ok(Self {
            grammar_string,
            root_rule,
        })
    }

    pub fn build_sampler(&self, model: &LlamaModel) -> Result<LlamaSampler> {
        LlamaSampler::grammar(model, &self.grammar_string, &self.root_rule)
            .map_err(|err| anyhow!("Failed to initialize grammar sampler: {err}"))
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use paddler_types::grammar_constraint::GrammarConstraint;

    use super::ResolvedGrammarSampler;

    #[test]
    fn new_with_gbnf_creates_correct_grammar() -> Result<()> {
        let constraint = GrammarConstraint::Gbnf {
            grammar: "root ::= \"yes\" | \"no\"".to_owned(),
            root: "my_root".to_owned(),
        };

        let sampler = ResolvedGrammarSampler::new(&constraint)?;

        assert_eq!(sampler.grammar_string, "root ::= \"yes\" | \"no\"");
        assert_eq!(sampler.root_rule, "my_root");

        Ok(())
    }

    #[test]
    fn new_with_json_schema_resolves_to_gbnf() -> Result<()> {
        let constraint = GrammarConstraint::JsonSchema {
            schema: r#"{"type": "object", "properties": {"name": {"type": "string"}}}"#
                .to_owned(),
        };

        let sampler = ResolvedGrammarSampler::new(&constraint)?;

        assert!(!sampler.grammar_string.is_empty());
        assert_eq!(sampler.root_rule, "root");

        Ok(())
    }

    #[test]
    fn returns_error_for_invalid_json_schema() {
        let constraint = GrammarConstraint::JsonSchema {
            schema: "not valid json".to_owned(),
        };

        let result = ResolvedGrammarSampler::new(&constraint);

        assert!(result.is_err());
    }
}
