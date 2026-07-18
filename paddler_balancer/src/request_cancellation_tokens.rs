use anyhow::Result;
use anyhow::anyhow;
use dashmap::DashMap;
use log::debug;
use tokio_util::sync::CancellationToken;

#[derive(Default)]
pub struct RequestCancellationTokens {
    cancellation_tokens: DashMap<String, CancellationToken>,
}

impl RequestCancellationTokens {
    pub fn register(
        &self,
        request_id: String,
        connection_close: &CancellationToken,
    ) -> Result<CancellationToken> {
        if self.cancellation_tokens.contains_key(&request_id) {
            return Err(anyhow!(
                "Cancellation token for request_id {request_id} already exists"
            ));
        }

        let request_close = connection_close.child_token();

        self.cancellation_tokens
            .insert(request_id, request_close.clone());

        Ok(request_close)
    }

    pub fn cancel(&self, request_id: &str) {
        match self.cancellation_tokens.get(request_id) {
            Some(cancellation_token) => cancellation_token.cancel(),
            None => debug!(
                "Received a stop request for an unknown or already finished request: {request_id:?}"
            ),
        }
    }

    pub fn deregister(&self, request_id: &str) {
        self.cancellation_tokens.remove(request_id);
    }
}

#[cfg(test)]
mod tests {
    use super::RequestCancellationTokens;
    use tokio_util::sync::CancellationToken;

    #[test]
    fn cancelling_one_request_leaves_the_other_requests_running() {
        let connection_close = CancellationToken::new();
        let request_cancellation_tokens = RequestCancellationTokens::default();

        let cancelled_request = request_cancellation_tokens
            .register("cancelled".to_owned(), &connection_close)
            .unwrap();
        let kept_request = request_cancellation_tokens
            .register("kept".to_owned(), &connection_close)
            .unwrap();

        request_cancellation_tokens.cancel("cancelled");

        assert!(cancelled_request.is_cancelled());
        assert!(!kept_request.is_cancelled());
    }

    #[test]
    fn closing_the_connection_cancels_every_registered_request() {
        let connection_close = CancellationToken::new();
        let request_cancellation_tokens = RequestCancellationTokens::default();

        let first_request = request_cancellation_tokens
            .register("first".to_owned(), &connection_close)
            .unwrap();
        let second_request = request_cancellation_tokens
            .register("second".to_owned(), &connection_close)
            .unwrap();

        connection_close.cancel();

        assert!(first_request.is_cancelled());
        assert!(second_request.is_cancelled());
    }

    #[test]
    fn a_request_registered_on_a_closed_connection_is_already_cancelled() {
        let connection_close = CancellationToken::new();
        let request_cancellation_tokens = RequestCancellationTokens::default();

        connection_close.cancel();

        assert!(
            request_cancellation_tokens
                .register("late".to_owned(), &connection_close)
                .unwrap()
                .is_cancelled()
        );
    }

    #[test]
    fn registering_a_duplicate_request_id_is_rejected_and_leaves_the_original_cancellable() {
        let connection_close = CancellationToken::new();
        let request_cancellation_tokens = RequestCancellationTokens::default();

        let original_request = request_cancellation_tokens
            .register("in_flight".to_owned(), &connection_close)
            .unwrap();

        assert!(
            request_cancellation_tokens
                .register("in_flight".to_owned(), &connection_close)
                .is_err()
        );

        request_cancellation_tokens.cancel("in_flight");

        assert!(original_request.is_cancelled());
    }

    #[test]
    fn cancelling_a_deregistered_request_does_nothing() {
        let connection_close = CancellationToken::new();
        let request_cancellation_tokens = RequestCancellationTokens::default();

        let finished_request = request_cancellation_tokens
            .register("finished".to_owned(), &connection_close)
            .unwrap();

        request_cancellation_tokens.deregister("finished");
        request_cancellation_tokens.cancel("finished");

        assert!(!finished_request.is_cancelled());
    }
}
