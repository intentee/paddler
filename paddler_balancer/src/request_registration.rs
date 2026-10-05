use tokio_util::sync::CancellationToken;

pub enum RequestRegistration {
    DuplicateRequestId,
    Registered(CancellationToken),
}
