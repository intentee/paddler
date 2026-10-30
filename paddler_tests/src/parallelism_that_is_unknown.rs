use std::io;
use std::num::NonZeroUsize;

use paddler_agent_runtime::reports_available_parallelism::ReportsAvailableParallelism;

pub struct ParallelismThatIsUnknown;

impl ReportsAvailableParallelism for ParallelismThatIsUnknown {
    fn available_parallelism(&self) -> io::Result<NonZeroUsize> {
        Err(io::Error::other(
            "the machine hides how many threads it runs in parallel",
        ))
    }
}
