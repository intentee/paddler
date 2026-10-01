use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

use paddler_client::error::Result as ClientResult;
use paddler_client::inference_message_stream::InferenceMessageStream;

use crate::budgeted_inference_message_stream::BudgetedInferenceMessageStream;
use crate::cluster_harness_error::ClusterHarnessError;
use crate::connection_reservation::ConnectionReservation;

const PORTABLE_LISTEN_BACKLOG: usize = 128;

#[derive(Default)]
pub struct ConcurrentConnectionBudget {
    held_connections: Arc<AtomicUsize>,
}

impl ConcurrentConnectionBudget {
    pub fn hold_during<TOutput, TError, TRequest>(
        &self,
        request: TRequest,
    ) -> impl Future<Output = Result<TOutput, TError>> + Send + use<TOutput, TError, TRequest>
    where
        TError: From<ClusterHarnessError>,
        TRequest: Future<Output = Result<TOutput, TError>> + Send,
    {
        let connection_reservation = self.reserve();

        async move {
            let _connection_reservation = connection_reservation?;

            request.await
        }
    }

    pub fn hold_while_streaming<TRequest>(
        &self,
        request: TRequest,
    ) -> impl Future<Output = Result<InferenceMessageStream, ClusterHarnessError>> + Send + use<TRequest>
    where
        TRequest: Future<Output = ClientResult<InferenceMessageStream>> + Send,
    {
        let connection_reservation = self.reserve();

        async move {
            let connection_reservation = connection_reservation?;
            let inference_message_stream = request
                .await
                .map_err(ClusterHarnessError::InferenceRequestFailed)?;

            Ok(Box::pin(BudgetedInferenceMessageStream {
                connection_reservation,
                inference_message_stream,
            }) as InferenceMessageStream)
        }
    }

    pub fn reserve(&self) -> Result<ConnectionReservation, ClusterHarnessError> {
        self.held_connections
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |held_connections| {
                (held_connections < PORTABLE_LISTEN_BACKLOG).then_some(held_connections + 1)
            })
            .map_err(|held_connections| {
                ClusterHarnessError::ConcurrentConnectionsExceedPortableListenBacklog {
                    held_connections,
                    limit: PORTABLE_LISTEN_BACKLOG,
                }
            })?;

        Ok(ConnectionReservation {
            held_connections: self.held_connections.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use std::iter::repeat_with;
    use std::sync::atomic::Ordering;

    use super::ConcurrentConnectionBudget;
    use super::PORTABLE_LISTEN_BACKLOG;
    use crate::cluster_harness_error::ClusterHarnessError;

    #[test]
    fn refuses_a_connection_beyond_the_portable_listen_backlog() {
        let budget = ConcurrentConnectionBudget::default();
        let _held_reservations: Vec<_> = repeat_with(|| budget.reserve())
            .take(PORTABLE_LISTEN_BACKLOG)
            .collect::<Result<_, _>>()
            .unwrap();

        assert!(matches!(
            budget.reserve(),
            Err(
                ClusterHarnessError::ConcurrentConnectionsExceedPortableListenBacklog {
                    held_connections: PORTABLE_LISTEN_BACKLOG,
                    limit: PORTABLE_LISTEN_BACKLOG,
                }
            )
        ));
    }

    #[test]
    fn releasing_a_connection_frees_its_place_in_the_budget() {
        let budget = ConcurrentConnectionBudget::default();

        drop(budget.reserve().unwrap());

        let reservation = budget.reserve().unwrap();

        assert_eq!(reservation.held_connections.load(Ordering::Acquire), 1);
    }
}
