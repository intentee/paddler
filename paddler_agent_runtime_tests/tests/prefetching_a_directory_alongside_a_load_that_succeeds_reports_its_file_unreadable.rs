use std::io::ErrorKind;
use std::num::NonZeroUsize;

use tempfile::TempDir;

use paddler_agent_runtime::agent_runtime_error::AgentRuntimeError;
use paddler_agent_runtime::model_file_prefetch::ModelFilePrefetch;

#[test]
fn prefetching_a_directory_alongside_a_load_that_succeeds_reports_its_file_unreadable() {
    let model_directory = TempDir::new().expect("a temporary directory must be creatable");

    let prefetched_load = ModelFilePrefetch {
        model_path: model_directory.path(),
        stream_count: NonZeroUsize::MIN,
    }
    .alongside(|| Ok(()));

    assert!(matches!(
        prefetched_load,
        Err(AgentRuntimeError::ModelFilePrefetchFailed {
            model_path,
            source,
        }) if model_path == model_directory.path() && source.kind() == ErrorKind::IsADirectory
    ));
}
