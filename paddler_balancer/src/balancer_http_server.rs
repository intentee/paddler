pub enum BalancerHttpServer {
    Inference,
    Management,
    OpenAI,
    WebAdminPanel,
}

impl BalancerHttpServer {
    /// Number of actix worker threads each balancer HTTP server runs.
    ///
    /// One worker multiplexes thousands of connections as async tasks (a websocket is a suspended
    /// task, not a thread), so these counts size CPU parallelism, not connection capacity. They are
    /// fixed per service load profile: inference and OpenAI-compat are client-facing request
    /// processors that do inline JSON/SSE work; management carries mostly-idle agent control
    /// sockets; the web admin panel serves static assets to a handful of human operators. Fixed
    /// values keep startup file-descriptor usage predictable regardless of the host CPU count.
    #[must_use]
    pub const fn worker_count(&self) -> usize {
        match self {
            Self::Inference | Self::OpenAI => 16,
            Self::Management => 4,
            Self::WebAdminPanel => 2,
        }
    }
}
