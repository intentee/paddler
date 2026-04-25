#![cfg(feature = "tests_that_use_compiled_paddler")]

use anyhow::Context as _;
use anyhow::Result;
use anyhow::anyhow;
use nix::sys::signal::Signal;
use nix::sys::signal::kill;
use nix::unistd::Pid;
use paddler_gui_tests::headless_display::HeadlessDisplay;
use paddler_gui_tests::spawn_gui_subprocess::spawn_gui_subprocess;
use paddler_gui_tests::spawn_gui_subprocess_params::SpawnGuiSubprocessParams;
use paddler_gui_tests::wait_for_log_line::wait_for_log_line;
use tokio::io::BufReader;

#[tokio::test(flavor = "multi_thread")]
async fn gui_exits_cleanly_on_sigterm_at_home_screen() -> Result<()> {
    let display = HeadlessDisplay::start().await?;

    let mut gui = spawn_gui_subprocess(SpawnGuiSubprocessParams {
        display_name: display.display_name().to_owned(),
        args: vec![],
    })?;

    let stderr = gui
        .stderr
        .take()
        .ok_or_else(|| anyhow!("paddler_gui stderr is not piped"))?;
    let mut stderr_reader = BufReader::new(stderr);
    let mut captured = Vec::new();

    wait_for_log_line(
        &mut stderr_reader,
        "paddler_gui: iced event loop ready",
        &mut captured,
    )
    .await?;

    let raw_pid = gui
        .id()
        .ok_or_else(|| anyhow!("paddler_gui process has no PID"))?;
    #[expect(clippy::cast_possible_wrap, reason = "PID values fit in i32")]
    let pid = Pid::from_raw(raw_pid as i32);

    kill(pid, Signal::SIGTERM).context("failed to send SIGTERM to paddler_gui")?;

    let exit_status = gui
        .wait()
        .await
        .context("failed to wait for paddler_gui exit")?;

    assert!(
        exit_status.success() || exit_status.code().is_some(),
        "paddler_gui terminated abnormally: {exit_status:?}"
    );

    drop(display);

    Ok(())
}
