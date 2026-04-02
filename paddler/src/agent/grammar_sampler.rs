use anyhow::Result;
use anyhow::anyhow;
use llama_cpp_bindings::model::LlamaModel;
use llama_cpp_bindings::sampling::LlamaSampler;
use paddler_types::grammar_constraint::GrammarConstraint;

use crate::agent::resolve_grammar_to_gbnf::resolve_grammar_to_gbnf;

pub struct GrammarSampler {
    sampler: LlamaSampler,
}

impl GrammarSampler {
    pub fn new(grammar_constraint: &GrammarConstraint, model: &LlamaModel) -> Result<Self> {
        let resolved = resolve_grammar_to_gbnf(grammar_constraint)?;

        let sampler = LlamaSampler::grammar(model, &resolved.grammar_string, &resolved.root_rule)
            .map_err(|err| anyhow!("Failed to initialize grammar sampler: {err}"))?;

        Ok(Self { sampler })
    }

    #[must_use]
    pub fn into_llama_sampler(self) -> LlamaSampler {
        self.sampler
    }
}
