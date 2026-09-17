use anyhow::Result;
use anyhow::anyhow;
use llama_cpp_bindings::model::LlamaModel;
use llama_cpp_bindings::sampling::LlamaSampler;
use paddler_messaging::grammar_constraint::GrammarConstraint;

use crate::grammar_engagement::GrammarEngagement;
use crate::request_grammar::RequestGrammar;
use crate::resolve_grammar_to_gbnf::resolve_grammar_to_gbnf;

pub struct GrammarSampler {
    engagement: GrammarEngagement,
    grammar_string: String,
    root_rule: String,
}

impl GrammarSampler {
    pub fn new(
        grammar_constraint: &GrammarConstraint,
        engagement: GrammarEngagement,
    ) -> Result<Self> {
        let resolved = resolve_grammar_to_gbnf(grammar_constraint)?;

        Ok(Self {
            engagement,
            grammar_string: resolved.grammar_string,
            root_rule: resolved.root_rule,
        })
    }

    pub fn into_request_grammar(self, model: &LlamaModel) -> Result<RequestGrammar> {
        let sampler = LlamaSampler::grammar(model, &self.grammar_string, &self.root_rule)
            .map_err(|err| anyhow!("Failed to initialize grammar sampler: {err}"))?;

        Ok(RequestGrammar::new(sampler, self.engagement))
    }
}
