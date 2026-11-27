use llama_cpp_decision::decision_prompt::DecisionPrompt;
use llama_cpp_mtmd::mtmd_context::MtmdContext;

pub struct PlannedDecisionPrompt {
    pub media_starts: Vec<usize>,
    pub prompt: DecisionPrompt,
}

impl PlannedDecisionPrompt {
    #[must_use]
    pub fn new(prompt: DecisionPrompt, projector: Option<&MtmdContext>) -> Self {
        let media_starts = projector.map_or_else(Vec::new, |projector| {
            (0..prompt.tokens().len())
                .filter(|position| prompt.media_chunk(*position, projector).is_some())
                .collect()
        });

        Self {
            media_starts,
            prompt,
        }
    }

    #[must_use]
    pub fn text_end_after(&self, token_index: usize, span_end: usize) -> usize {
        let next_media_start = self
            .media_starts
            .partition_point(|media_start| *media_start <= token_index);

        self.media_starts
            .get(next_media_start)
            .copied()
            .filter(|media_start| *media_start < span_end)
            .unwrap_or(span_end)
    }
}
