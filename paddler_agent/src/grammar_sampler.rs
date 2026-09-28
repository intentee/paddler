use llama_cpp_bindings::error::GrammarError;
use llama_cpp_bindings::error::JsonSchemaToGrammarError;
use llama_cpp_bindings::json_schema_to_grammar;
use llama_cpp_bindings::model::LlamaModel;
use llama_cpp_bindings::sampling::LlamaSampler;
use paddler_messaging::grammar_constraint::GrammarConstraint;

#[derive(Debug, Eq, PartialEq)]
pub struct GrammarSampler {
    grammar_string: String,
    root_rule: String,
}

impl GrammarSampler {
    pub fn new(grammar_constraint: GrammarConstraint) -> Result<Self, JsonSchemaToGrammarError> {
        match grammar_constraint {
            GrammarConstraint::Gbnf { grammar, root } => Ok(Self {
                grammar_string: grammar,
                root_rule: root,
            }),
            GrammarConstraint::JsonSchema { schema } => Ok(Self {
                grammar_string: json_schema_to_grammar(&schema)?,
                root_rule: "root".to_owned(),
            }),
        }
    }

    pub fn into_llama_sampler(self, model: &LlamaModel) -> Result<LlamaSampler, GrammarError> {
        LlamaSampler::grammar(model, &self.grammar_string, &self.root_rule)
    }
}

#[cfg(test)]
mod tests {
    use std::mem::discriminant;

    use llama_cpp_bindings::error::JsonSchemaToGrammarError;
    use paddler_messaging::grammar_constraint::GrammarConstraint;

    use super::GrammarSampler;

    #[test]
    fn keeps_gbnf_grammar_and_root_rule() {
        assert_eq!(
            GrammarSampler::new(GrammarConstraint::Gbnf {
                grammar: "root ::= \"yes\" | \"no\"".to_owned(),
                root: "root".to_owned(),
            }),
            Ok(GrammarSampler {
                grammar_string: "root ::= \"yes\" | \"no\"".to_owned(),
                root_rule: "root".to_owned(),
            })
        );
    }

    #[test]
    fn converts_json_schema_to_gbnf_rooted_at_root() {
        let grammar_sampler = GrammarSampler::new(GrammarConstraint::JsonSchema {
            schema: r#"{"type": "object", "properties": {"name": {"type": "string"}}}"#.to_owned(),
        })
        .unwrap();

        assert!(!grammar_sampler.grammar_string.is_empty());
        assert_eq!(grammar_sampler.root_rule, "root");
    }

    #[test]
    fn rejects_invalid_json_schema() {
        assert_eq!(
            GrammarSampler::new(GrammarConstraint::JsonSchema {
                schema: "not valid json".to_owned(),
            })
            .err()
            .map(|error| discriminant(&error)),
            Some(discriminant(&JsonSchemaToGrammarError::Reported {
                message: String::new(),
            }))
        );
    }
}
