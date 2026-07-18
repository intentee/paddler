use crate::request_cancellation_token_guard::RequestCancellationTokenGuard;

pub enum RequestCancellationRegistration {
    DuplicateRequestId,
    Registered(RequestCancellationTokenGuard),
}
