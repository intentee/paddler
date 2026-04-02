use anyhow::Result;
use anyhow::anyhow;
use llama_cpp_bindings::model::LlamaModel;
use llama_cpp_bindings::sampling::LlamaSampler;
use paddler_types::grammar_constraint::GrammarConstraint;

use crate::agent::resolve_grammar_to_gbnf::resolve_grammar_to_gbnf;

pub struct GrammarSampler {
    grammar_string: String,
    root_rule: String,
}

impl GrammarSampler {
    pub fn new(grammar_constraint: &GrammarConstraint) -> Result<Self> {
        let resolved = resolve_grammar_to_gbnf(grammar_constraint)?;

        Ok(Self {
            grammar_string: resolved.grammar_string,
            root_rule: resolved.root_rule,
        })
    }

    pub fn into_llama_sampler(self, model: &LlamaModel) -> Result<LlamaSampler> {
        LlamaSampler::grammar(model, &self.grammar_string, &self.root_rule)
            .map_err(|err| anyhow!("Failed to initialize grammar sampler: {err}"))
    }
}
