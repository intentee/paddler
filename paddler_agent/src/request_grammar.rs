use llama_cpp_bindings::SampledTokenSection;
use llama_cpp_bindings::sampling::LlamaSampler;

use crate::grammar_engagement::GrammarEngagement;

pub struct RequestGrammar {
    engagement: GrammarEngagement,
    sampler: LlamaSampler,
}

impl RequestGrammar {
    #[must_use]
    pub const fn new(sampler: LlamaSampler, engagement: GrammarEngagement) -> Self {
        Self {
            engagement,
            sampler,
        }
    }

    #[must_use]
    pub const fn constrains(&self, section: SampledTokenSection) -> bool {
        match self.engagement {
            GrammarEngagement::Immediate => true,
            GrammarEngagement::AfterReasoning => {
                matches!(section, SampledTokenSection::Content)
            }
        }
    }

    pub const fn sampler_for(&mut self, section: SampledTokenSection) -> Option<&mut LlamaSampler> {
        if self.constrains(section) {
            Some(&mut self.sampler)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use llama_cpp_bindings::SampledTokenSection;

    use super::GrammarEngagement;
    use super::RequestGrammar;

    const EVERY_SECTION: [SampledTokenSection; 4] = [
        SampledTokenSection::Pending,
        SampledTokenSection::Content,
        SampledTokenSection::Reasoning,
        SampledTokenSection::ToolCall,
    ];

    fn grammar(engagement: GrammarEngagement) -> RequestGrammar {
        RequestGrammar {
            engagement,
            sampler: llama_cpp_bindings::sampling::LlamaSampler::greedy()
                .expect("the greedy sampler must initialize"),
        }
    }

    #[test]
    fn an_immediate_grammar_constrains_every_section() {
        let request_grammar = grammar(GrammarEngagement::Immediate);

        for section in EVERY_SECTION {
            assert!(request_grammar.constrains(section));
        }
    }

    #[test]
    fn a_deferred_grammar_constrains_only_content() {
        let request_grammar = grammar(GrammarEngagement::AfterReasoning);

        for section in EVERY_SECTION {
            assert_eq!(
                request_grammar.constrains(section),
                matches!(section, SampledTokenSection::Content)
            );
        }
    }

    #[test]
    fn a_deferred_grammar_yields_no_sampler_while_reasoning() {
        let mut request_grammar = grammar(GrammarEngagement::AfterReasoning);

        assert!(
            request_grammar
                .sampler_for(SampledTokenSection::Reasoning)
                .is_none()
        );
        assert!(
            request_grammar
                .sampler_for(SampledTokenSection::Content)
                .is_some()
        );
    }
}
