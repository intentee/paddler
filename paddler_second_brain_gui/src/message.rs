#[derive(Debug, Clone)]
pub enum Message {
    StartBalancer,
    BalancerStopped,
    BalancerFailed(String),
}
