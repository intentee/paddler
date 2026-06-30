use anyhow::Result;

use crate::continuous_batch_request_state::ContinuousBatchRequestState;
use crate::continuous_batch_scheduler::contributions::Contributions;

pub fn run(
    contributions: Contributions,
    request_states: &mut [&mut ContinuousBatchRequestState],
) -> Result<()> {
    for contribution in contributions.generating {
        request_states[contribution.request_index]
            .apply_generating_contribution(contribution.batch_position);
    }

    for contribution in contributions.ingesting {
        request_states[contribution.request_index].apply_ingesting_contribution(
            contribution.chunk_size,
            contribution.is_last_chunk,
            contribution.last_batch_position,
        )?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use llama_cpp_bindings::token::LlamaToken;

    use super::run;
    use crate::continuous_batch_request_phase::ContinuousBatchRequestPhase;
    use crate::continuous_batch_request_state::ContinuousBatchRequestState;
    use crate::continuous_batch_scheduler::contributions::Contributions;
    use crate::continuous_batch_scheduler::generating_contribution::GeneratingContribution;
    use crate::continuous_batch_scheduler::ingesting_contribution::IngestingContribution;

    fn request_state(phase: ContinuousBatchRequestPhase) -> ContinuousBatchRequestState {
        ContinuousBatchRequestState {
            current_token_position: 0,
            i_batch: None,
            max_tokens: 64,
            pending_sampled_token: None,
            phase,
            prompt_tokens: vec![LlamaToken::new(1); 8],
            prompt_tokens_ingested: 0,
            sequence_id: 0,
        }
    }

    #[test]
    fn generating_contribution_is_routed_to_its_request_index() {
        let mut first = request_state(ContinuousBatchRequestPhase::Generating);
        let mut second = request_state(ContinuousBatchRequestPhase::Generating);
        let mut request_states = vec![&mut first, &mut second];

        let mut contributions = Contributions::default();
        contributions.generating.push(GeneratingContribution {
            request_index: 1,
            batch_position: 5,
        });

        run(contributions, &mut request_states).unwrap();

        assert_eq!(first.i_batch, None);
        assert_eq!(second.i_batch, Some(5));
    }

    #[test]
    fn ingesting_contribution_is_routed_to_its_request_index() {
        let mut first = request_state(ContinuousBatchRequestPhase::Ingesting);
        let mut second = request_state(ContinuousBatchRequestPhase::Ingesting);
        let mut request_states = vec![&mut first, &mut second];

        let mut contributions = Contributions::default();
        contributions.ingesting.push(IngestingContribution {
            request_index: 0,
            chunk_size: 3,
            is_last_chunk: true,
            last_batch_position: 7,
        });

        run(contributions, &mut request_states).unwrap();

        assert_eq!(first.prompt_tokens_ingested, 3);
        assert_eq!(first.i_batch, Some(7));
        assert_eq!(second.prompt_tokens_ingested, 0);
    }

    #[test]
    fn oversized_ingesting_chunk_propagates_the_error() {
        let mut only = request_state(ContinuousBatchRequestPhase::Ingesting);
        let mut request_states = vec![&mut only];

        let mut contributions = Contributions::default();
        contributions.ingesting.push(IngestingContribution {
            request_index: 0,
            chunk_size: usize::MAX,
            is_last_chunk: false,
            last_batch_position: 0,
        });

        assert!(run(contributions, &mut request_states).is_err());
    }
}
