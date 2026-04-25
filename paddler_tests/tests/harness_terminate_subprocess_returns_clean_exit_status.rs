use anyhow::Result;
use paddler_tests::terminate_subprocess::terminate_subprocess;
use std::process::Stdio;
use tokio::process::Command;

#[tokio::test(flavor = "multi_thread")]
async fn harness_terminate_subprocess_returns_clean_exit_status() -> Result<()> {
    let child = Command::new("sleep")
        .arg("60")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;

    let exit_status = terminate_subprocess(child).await?;

    assert!(
        !exit_status.success(),
        "sleep 60 receiving SIGTERM should not report success exit"
    );
    assert!(
        exit_status.code().is_none(),
        "SIGTERM-terminated process should not have a normal exit code"
    );

    Ok(())
}
