use tokio_util::sync::CancellationToken;

pub enum BalancerLaunch {
    NotRequested,
    Starting(CancellationToken),
}
