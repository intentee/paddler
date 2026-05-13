use crate::agent::continuous_batch_scheduler::generating_contribution::GeneratingContribution;
use crate::agent::continuous_batch_scheduler::ingesting_contribution::IngestingContribution;

#[derive(Default)]
pub struct Contributions {
    pub generating: Vec<GeneratingContribution>,
    pub ingesting: Vec<IngestingContribution>,
    pub current_batch_token_count: usize,
}

impl Contributions {
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.generating.is_empty() && self.ingesting.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::Contributions;
    use super::GeneratingContribution;
    use super::IngestingContribution;

    #[test]
    fn default_contributions_are_empty() {
        let contributions = Contributions::default();

        assert!(contributions.is_empty());
        assert_eq!(contributions.current_batch_token_count, 0);
    }

    #[test]
    fn contributions_with_generating_entry_is_not_empty() {
        let mut contributions = Contributions::default();
        contributions.generating.push(GeneratingContribution {
            request_index: 0,
            batch_position: 1,
        });

        assert!(!contributions.is_empty());
    }

    #[test]
    fn contributions_with_ingesting_entry_is_not_empty() {
        let mut contributions = Contributions::default();
        contributions.ingesting.push(IngestingContribution {
            request_index: 0,
            chunk_size: 4,
            is_last_chunk: false,
            last_batch_position: 3,
        });

        assert!(!contributions.is_empty());
    }
}
