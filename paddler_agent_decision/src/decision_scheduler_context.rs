use crate::kev_option_scorer::KevOptionScorer;

pub struct DecisionSchedulerContext {
    pub agent_name: Option<String>,
    pub option_scorer: KevOptionScorer,
}
