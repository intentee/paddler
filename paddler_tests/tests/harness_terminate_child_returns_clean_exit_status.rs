#![cfg(feature = "tests_that_use_compiled_paddler")]

use std::process::Stdio;

use anyhow::Result;
use paddler_tests::paddler_command::paddler_command;
use paddler_tests::terminate_child::terminate_child;

#[tokio::test(flavor = "multi_thread")]
async fn harness_terminate_child_returns_clean_exit_status() -> Result<()> {
    let mut child = paddler_command()
        .arg("agent")
        .arg("--management-addr")
        .arg("127.0.0.1:1")
        .arg("--name")
        .arg("harness-terminate-child-test")
        .arg("--slots")
        .arg("1")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;

    terminate_child(&mut child)?;

    let exit_status = child.wait().await?;

    assert!(
        !exit_status.success(),
        "terminated process must not report success exit; got {exit_status:?}"
    );

    #[cfg(unix)]
    assert!(
        exit_status.code().is_none(),
        "SIGTERM-terminated process must have no normal exit code on Unix; got {exit_status:?}"
    );

    Ok(())
}
