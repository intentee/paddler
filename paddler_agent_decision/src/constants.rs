use crate::decision_delimiters::DecisionDelimiters;

pub const KEV_DECISION_DELIMITERS: DecisionDelimiters<'static> = DecisionDelimiters {
    decide: "<|fim_suffix|>",
    option_end: "<|box_end|>",
    option_start: "<|box_start|>",
    question: "<|fim_middle|>",
    state: "<|fim_prefix|>",
};
