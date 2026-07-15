use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use crate::request_cancellation_tokens::RequestCancellationTokens;

pub struct RequestCancellationTokenGuard {
    pub cancellation_token: CancellationToken,
    request_cancellation_tokens: Arc<RequestCancellationTokens>,
    request_id: String,
}

impl RequestCancellationTokenGuard {
    #[must_use]
    pub fn register(
        connection_close: &CancellationToken,
        request_cancellation_tokens: Arc<RequestCancellationTokens>,
        request_id: String,
    ) -> Self {
        let cancellation_token =
            request_cancellation_tokens.register(request_id.clone(), connection_close);

        Self {
            cancellation_token,
            request_cancellation_tokens,
            request_id,
        }
    }
}

impl Drop for RequestCancellationTokenGuard {
    fn drop(&mut self) {
        self.request_cancellation_tokens
            .deregister(&self.request_id);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use tokio_util::sync::CancellationToken;

    use super::RequestCancellationTokenGuard;
    use crate::request_cancellation_tokens::RequestCancellationTokens;

    #[test]
    fn dropping_the_guard_stops_the_request_from_being_cancellable() {
        let connection_close = CancellationToken::new();
        let request_cancellation_tokens = Arc::new(RequestCancellationTokens::default());

        let guard = RequestCancellationTokenGuard::register(
            &connection_close,
            request_cancellation_tokens.clone(),
            "finished".to_owned(),
        );
        let cancellation_token = guard.cancellation_token.clone();

        drop(guard);

        request_cancellation_tokens.cancel("finished");

        assert!(!cancellation_token.is_cancelled());
    }

    #[test]
    fn a_registered_request_is_cancelled_through_the_collection() {
        let connection_close = CancellationToken::new();
        let request_cancellation_tokens = Arc::new(RequestCancellationTokens::default());

        let guard = RequestCancellationTokenGuard::register(
            &connection_close,
            request_cancellation_tokens.clone(),
            "in_flight".to_owned(),
        );

        request_cancellation_tokens.cancel("in_flight");

        assert!(guard.cancellation_token.is_cancelled());
    }
}
