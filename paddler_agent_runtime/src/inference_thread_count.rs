use std::cmp::max;
use std::num::NonZeroUsize;

use crate::agent_runtime_error::AgentRuntimeError;
use crate::reports_available_parallelism::ReportsAvailableParallelism;

const LOGICAL_CORES_PER_PHYSICAL_CORE: i32 = 2;
const MINIMUM_THREAD_COUNT: i32 = 2;

fn thread_count_for_logical_cores(logical_cores: NonZeroUsize) -> Result<i32, AgentRuntimeError> {
    let logical_cores =
        i32::try_from(logical_cores.get()).map_err(AgentRuntimeError::ThreadCountOutOfRange)?;

    Ok(max(
        MINIMUM_THREAD_COUNT,
        logical_cores / LOGICAL_CORES_PER_PHYSICAL_CORE,
    ))
}

pub fn inference_thread_count(
    parallelism: &dyn ReportsAvailableParallelism,
) -> Result<i32, AgentRuntimeError> {
    thread_count_for_logical_cores(
        parallelism
            .available_parallelism()
            .map_err(AgentRuntimeError::AvailableParallelismUnknown)?,
    )
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroUsize;

    use super::MINIMUM_THREAD_COUNT;
    use super::thread_count_for_logical_cores;

    #[test]
    fn never_runs_fewer_than_the_minimum_threads() {
        assert_eq!(
            thread_count_for_logical_cores(NonZeroUsize::MIN).unwrap(),
            MINIMUM_THREAD_COUNT
        );
    }

    #[test]
    fn reports_a_parallelism_llama_cpp_cannot_address() {
        assert_eq!(
            thread_count_for_logical_cores(NonZeroUsize::MAX)
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
