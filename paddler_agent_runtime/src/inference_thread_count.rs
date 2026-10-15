use std::cmp::max;

use crate::agent_runtime_error::AgentRuntimeError;
use crate::reports_available_parallelism::ReportsAvailableParallelism;

const LOGICAL_CORES_PER_PHYSICAL_CORE: i32 = 2;
const MINIMUM_THREAD_COUNT: i32 = 2;

pub fn inference_thread_count(
    parallelism: &dyn ReportsAvailableParallelism,
) -> Result<i32, AgentRuntimeError> {
    let logical_cores = i32::try_from(
        parallelism
            .available_parallelism()
            .map_err(AgentRuntimeError::AvailableParallelismUnknown)?
            .get(),
    )
    .map_err(AgentRuntimeError::ThreadCountOutOfRange)?;

    Ok(max(
        MINIMUM_THREAD_COUNT,
        logical_cores / LOGICAL_CORES_PER_PHYSICAL_CORE,
    ))
}

#[cfg(test)]
mod tests {
    use std::io;
    use std::num::NonZeroUsize;

    use super::MINIMUM_THREAD_COUNT;
    use super::inference_thread_count;
    use crate::reports_available_parallelism::ReportsAvailableParallelism;
    use crate::system_parallelism::SystemParallelism;

    struct ParallelismBeyondThreadCountRange;

    impl ReportsAvailableParallelism for ParallelismBeyondThreadCountRange {
        fn available_parallelism(&self) -> io::Result<NonZeroUsize> {
            Ok(NonZeroUsize::MAX)
        }
    }

    #[test]
    fn never_runs_fewer_than_the_minimum_threads() {
        assert!(inference_thread_count(&SystemParallelism).unwrap() >= MINIMUM_THREAD_COUNT);
    }

    #[test]
    fn reports_a_parallelism_llama_cpp_cannot_address() {
        assert_eq!(
            inference_thread_count(&ParallelismBeyondThreadCountRange)
                .err()
                .as_ref()
                .map(ToString::to_string),
            Some("the machine runs more threads in parallel than llama.cpp can address".to_owned())
        );
    }
}
