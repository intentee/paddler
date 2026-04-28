#![cfg(all(target_os = "linux", feature = "tests_that_use_compiled_paddler"))]

use std::os::unix::process::ExitStatusExt as _;

use anyhow::Context as _;
use anyhow::Result;
use anyhow::anyhow;
use paddler_gui_tests::headless_display::HeadlessDisplay;
use paddler_gui_tests::spawn_gui_subprocess::spawn_gui_subprocess;
use paddler_gui_tests::spawn_gui_subprocess_params::SpawnGuiSubprocessParams;
use paddler_tests::terminate_child::terminate_child;

const SIGTERM_NUMBER: i32 = 15;

#[tokio::test(flavor = "multi_thread")]
async fn gui_exits_cleanly_on_sigterm_at_home_screen() -> Result<()> {
    let display = HeadlessDisplay::start().await?;

    let mut gui = spawn_gui_subprocess(SpawnGuiSubprocessParams {
        display_name: display.display_name().to_owned(),
        args: vec![],
    })?;

    if gui.id().is_none() {
        return Err(anyhow!("paddler_gui process has no PID"));
    }

    terminate_child(&mut gui).context("failed to send termination signal to paddler_gui")?;

    let exit_status = gui
        .wait()
        .await
        .context("failed to wait for paddler_gui exit")?;

    let exited_via_sigterm = exit_status.signal() == Some(SIGTERM_NUMBER);
    let exited_via_code = exit_status.code().is_some();

    assert!(
        exited_via_sigterm || exited_via_code,
        "paddler_gui terminated abnormally (expected SIGTERM or exit code): {exit_status:?}"
    );

    drop(display);

    Ok(())
}
