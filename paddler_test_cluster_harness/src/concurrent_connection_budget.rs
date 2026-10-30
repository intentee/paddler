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
            .try_update(Ordering::AcqRel, Ordering::Acquire, |held_connections| {
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
    use std::future::pending;
    use std::iter::repeat_with;
    use std::sync::atomic::Ordering;

    use futures_util::FutureExt as _;

    use paddler_client::error::Result as ClientResult;
    use paddler_client::inference_message_stream::InferenceMessageStream;

    use super::ConcurrentConnectionBudget;
    use super::PORTABLE_LISTEN_BACKLOG;
    use crate::cluster_harness_error::ClusterHarnessError;
    use crate::connection_reservation::ConnectionReservation;

    fn hold_every_connection(budget: &ConcurrentConnectionBudget) -> Vec<ConnectionReservation> {
        repeat_with(|| budget.reserve())
            .take(PORTABLE_LISTEN_BACKLOG)
            .collect::<Result<_, _>>()
            .unwrap()
    }

    #[test]
    fn refuses_a_connection_beyond_the_portable_listen_backlog() {
        let budget = ConcurrentConnectionBudget::default();
        let _held_reservations = hold_every_connection(&budget);

        assert_eq!(
            budget.reserve().err().map(|refusal| refusal.to_string()),
            Some(
                ClusterHarnessError::ConcurrentConnectionsExceedPortableListenBacklog {
                    held_connections: PORTABLE_LISTEN_BACKLOG,
                    limit: PORTABLE_LISTEN_BACKLOG,
                }
                .to_string()
            )
        );
    }

    #[test]
    fn releasing_a_connection_frees_its_place_in_the_budget() {
        let budget = ConcurrentConnectionBudget::default();

        drop(budget.reserve().unwrap());

        let reservation = budget.reserve().unwrap();

        assert_eq!(reservation.held_connections.load(Ordering::Acquire), 1);
    }

    #[test]
    fn a_request_beyond_the_budget_is_refused_before_it_runs() {
        let budget = ConcurrentConnectionBudget::default();
        let _held_reservations = hold_every_connection(&budget);

        assert_eq!(
            budget
                .hold_during(pending::<Result<(), ClusterHarnessError>>())
                .now_or_never()
                .map(|refused_request| refused_request.map_err(|refusal| refusal.to_string())),
            Some(Err(
                ClusterHarnessError::ConcurrentConnectionsExceedPortableListenBacklog {
                    held_connections: PORTABLE_LISTEN_BACKLOG,
                    limit: PORTABLE_LISTEN_BACKLOG,
                }
                .to_string()
            ))
        );
    }

    #[test]
    fn a_stream_beyond_the_budget_is_refused_before_it_runs() {
        let budget = ConcurrentConnectionBudget::default();
        let _held_reservations = hold_every_connection(&budget);

        assert_eq!(
            budget
                .hold_while_streaming(pending::<ClientResult<InferenceMessageStream>>())
                .now_or_never()
                .map(|refused_stream| refused_stream.err().map(|refusal| refusal.to_string())),
            Some(Some(
                ClusterHarnessError::ConcurrentConnectionsExceedPortableListenBacklog {
                    held_connections: PORTABLE_LISTEN_BACKLOG,
                    limit: PORTABLE_LISTEN_BACKLOG,
                }
                .to_string()
            ))
        );
    }
}
