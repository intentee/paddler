#[derive(Debug, thiserror::Error)]
pub enum TestcontainerError {
    #[error(
        "docker host {domain:?} is neither an IP address nor localhost; the balancer's mapped ports must be reachable directly from the test process"
    )]
    NonLocalDockerHost { domain: String },
}
