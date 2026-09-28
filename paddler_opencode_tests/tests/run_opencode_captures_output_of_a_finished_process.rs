use std::path::Path;
use std::time::Duration;

use paddler_opencode_tests::run_opencode::run_opencode;

use crate::unreachable_api_opencode_test_project::unreachable_api_opencode_test_project;

const STUB_BINARY_TIMEOUT: Duration = Duration::from_secs(30);

#[tokio::test]
async fn run_opencode_captures_output_of_a_finished_process() {
    let outcome = run_opencode(
        Path::new(env!("CARGO_BIN_EXE_opencode_stub")),
        &unreachable_api_opencode_test_project()
            .expect("an OpenCode test project must be creatable"),
        "paddler-probe-argument",
        STUB_BINARY_TIMEOUT,
    )
    .await
    .expect("the OpenCode stub must run to completion");

    assert!(outcome.exit_success);
    assert!(outcome.stdout.contains("paddler-probe-argument"));
}
