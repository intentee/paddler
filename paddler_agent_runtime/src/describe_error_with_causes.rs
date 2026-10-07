use std::error::Error;

pub fn describe_error_with_causes(error: &dyn Error) -> String {
    let mut description = error.to_string();
    let mut cause = error.source();

    while let Some(current_cause) = cause {
        description = format!("{description}: {current_cause}");
        cause = current_cause.source();
    }

    description
}

#[cfg(test)]
mod tests {
    use std::io;

    use super::describe_error_with_causes;
    use crate::agent_runtime_error::AgentRuntimeError;

    #[test]
    fn appends_every_cause_to_the_description() {
        let error = AgentRuntimeError::AvailableParallelismUnknown(io::Error::other(
            "cgroup quota is unreadable",
        ));

        assert_eq!(
            describe_error_with_causes(&error),
            "unable to determine how many threads this machine runs in parallel: cgroup quota is unreadable"
        );
    }
}
