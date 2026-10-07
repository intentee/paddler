use std::cmp::max;
use std::thread::available_parallelism;

use crate::agent_runtime_error::AgentRuntimeError;

const LOGICAL_CORES_PER_PHYSICAL_CORE: i32 = 2;
const MINIMUM_THREAD_COUNT: i32 = 2;

pub fn inference_thread_count() -> Result<i32, AgentRuntimeError> {
    let logical_cores = i32::try_from(
        available_parallelism()
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
    use super::MINIMUM_THREAD_COUNT;
    use super::inference_thread_count;

    #[test]
    fn never_runs_fewer_than_the_minimum_threads() {
        assert!(inference_thread_count().unwrap() >= MINIMUM_THREAD_COUNT);
    }
}
