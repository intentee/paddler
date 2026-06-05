use thiserror::Error;

#[derive(Debug, Error)]
pub enum FileDescriptorLimitError {
    #[error(
        "open file-descriptor limit is too low for the balancer: {soft_limit} available, at least \
         {required} required; raise it with `ulimit -n {required}` (or higher) and restart"
    )]
    InsufficientDescriptors { soft_limit: u64, required: u64 },

    #[error("unable to read the open file-descriptor limit (RLIMIT_NOFILE): {message}")]
    UnableToReadLimit { message: String },
}
