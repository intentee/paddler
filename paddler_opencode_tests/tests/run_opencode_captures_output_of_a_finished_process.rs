use std::path::Path;

use paddler_opencode_tests::run_opencode::run_opencode;

use crate::unreachable_api_opencode_test_project::unreachable_api_opencode_test_project;

#[tokio::test]
async fn run_opencode_captures_output_of_a_finished_process() {
    let project = unreachable_api_opencode_test_project();
    let outcome = run_opencode(
        Path::new(env!("CARGO_BIN_EXE_opencode_stub")),
        &project,
        "paddler-probe-argument",
    )
    .await
    .expect("the OpenCode stub must run to completion");

    assert!(outcome.exit_success);
    assert_eq!(
        outcome.stdout,
        format!(
            "{}\n{:?}\n{:?}\n{:?}\nrun paddler-probe-argument --standalone --auto --model {}\n",
            project
                .directory_path()
                .canonicalize()
                .expect("the project directory must resolve")
                .display(),
            Some(project.directory_path().as_os_str()),
            Some(project.home_path().as_os_str()),
            Some(project.home_path().join(".config").as_os_str()),
            project.model_reference()
        )
    );
}
