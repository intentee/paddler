use std::cmp::max;
use std::num::NonZeroUsize;

use crate::agent_runtime_error::AgentRuntimeError;

const LOGICAL_CORES_PER_PHYSICAL_CORE: i32 = 2;
const MINIMUM_THREAD_COUNT: i32 = 2;

/// # Errors
///
/// Returns [`AgentRuntimeError::ThreadCountOutOfRange`] when the count of logical cores does not fit the thread count llama.cpp takes.
pub fn inference_thread_count(logical_cores: NonZeroUsize) -> Result<i32, AgentRuntimeError> {
    let logical_cores =
        i32::try_from(logical_cores.get()).map_err(AgentRuntimeError::ThreadCountOutOfRange)?;

    Ok(max(
        MINIMUM_THREAD_COUNT,
        logical_cores / LOGICAL_CORES_PER_PHYSICAL_CORE,
    ))
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroUsize;

    use super::MINIMUM_THREAD_COUNT;
    use super::inference_thread_count;

    #[test]
    fn never_runs_fewer_than_the_minimum_threads() {
        assert_eq!(
            inference_thread_count(NonZeroUsize::MIN).unwrap(),
            MINIMUM_THREAD_COUNT
        );
    }

    #[test]
    fn reports_a_parallelism_llama_cpp_cannot_address() {
        assert_eq!(
            inference_thread_count(NonZeroUsize::MAX)
                .err()
                .as_ref()
                .map(ToString::to_string),
            Some(
                "the machine runs more threads in parallel than llama.cpp can address: number too large to fit in target type"
                    .to_owned()
            )
        );
    }
}
