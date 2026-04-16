#![cfg(feature = "tests_that_use_headless_wayland")]

use std::os::unix::process::ExitStatusExt as _;
use std::path::Path;
use std::process::Child;
use std::process::Command;
use std::process::ExitStatus;
use std::process::Stdio;
use std::thread;
use std::time::Duration;
use std::time::Instant;

use anyhow::Context as _;
use anyhow::Result;
use anyhow::anyhow;
use nix::sys::signal::Signal;
use nix::sys::signal::kill;
use nix::unistd::Pid;
use tempfile::TempDir;

const COMPOSITOR_READY_DEADLINE: Duration = Duration::from_secs(10);
const GUI_EVENT_LOOP_DEADLINE: Duration = Duration::from_secs(10);
const SHUTDOWN_DEADLINE: Duration = Duration::from_secs(5);
const POLL_INTERVAL: Duration = Duration::from_millis(50);

fn spawn_weston(runtime_dir: &Path, socket_name: &str) -> Result<Child> {
    Command::new("weston")
        .arg("--backend=headless")
        .arg(format!("--socket={socket_name}"))
        .env("XDG_RUNTIME_DIR", runtime_dir)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .context("failed to spawn weston; ensure it is on PATH (nix-shell paddler_second_brain_gui/shell.nix)")
}

fn wait_for_socket(socket_path: &Path, weston: &mut Child) -> Result<()> {
    let deadline = Instant::now() + COMPOSITOR_READY_DEADLINE;

    while Instant::now() < deadline {
        if socket_path.exists() {
            return Ok(());
        }
        if let Some(status) = weston.try_wait()? {
            return Err(anyhow!(
                "weston exited before creating socket {}: {status}",
                socket_path.display()
            ));
        }

        thread::sleep(POLL_INTERVAL);
    }

    Err(anyhow!(
        "weston did not create socket {} within {:?}",
        socket_path.display(),
        COMPOSITOR_READY_DEADLINE
    ))
}

fn spawn_gui(runtime_dir: &Path, socket_name: &str) -> Result<Child> {
    Command::new(env!("CARGO_BIN_EXE_paddler_second_brain_gui"))
        .env("XDG_RUNTIME_DIR", runtime_dir)
        .env("WAYLAND_DISPLAY", socket_name)
        .env_remove("DISPLAY")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .context("failed to spawn paddler_second_brain_gui binary")
}

fn ensure_gui_reached_event_loop(gui: &mut Child) -> Result<()> {
    let deadline = Instant::now() + GUI_EVENT_LOOP_DEADLINE;

    while Instant::now() < deadline {
        if let Some(status) = gui.try_wait()? {
            return Err(anyhow!(
                "paddler_second_brain_gui exited before reaching event loop: {status}"
            ));
        }

        thread::sleep(POLL_INTERVAL);
    }

    Ok(())
}

fn terminate_and_wait(child: &mut Child) -> Result<ExitStatus> {
    let raw_pid = child
        .id()
        .try_into()
        .context("child pid does not fit in i32")?;
    let pid = Pid::from_raw(raw_pid);
    kill(pid, Signal::SIGTERM).context("failed to send SIGTERM")?;

    let deadline = Instant::now() + SHUTDOWN_DEADLINE;

    while Instant::now() < deadline {
        if let Some(status) = child.try_wait()? {
            return Ok(status);
        }

        thread::sleep(POLL_INTERVAL);
    }
    child
        .kill()
        .context("failed to SIGKILL child after SIGTERM timeout")?;

    child
        .wait()
        .context("failed to wait for child after SIGKILL")
}

fn assert_terminated_cleanly(status: ExitStatus, process_label: &str) -> Result<()> {
    if status.signal() == Some(Signal::SIGTERM as i32) {
        return Ok(());
    }
    if status.success() {
        return Ok(());
    }

    Err(anyhow!(
        "{process_label} did not terminate cleanly: {status:?}"
    ))
}

#[test]
fn application_reaches_event_loop() -> Result<()> {
    let runtime_dir = TempDir::new().context("failed to create runtime dir for wayland socket")?;
    let socket_name = format!("wayland-paddler-{}", std::process::id());
    let socket_path = runtime_dir.path().join(&socket_name);

    let mut weston = spawn_weston(runtime_dir.path(), &socket_name)?;
    let wait_result = wait_for_socket(&socket_path, &mut weston);
    if let Err(err) = wait_result {
        let _ = terminate_and_wait(&mut weston);

        return Err(err);
    }

    let mut gui = spawn_gui(runtime_dir.path(), &socket_name)?;
    let event_loop_result = ensure_gui_reached_event_loop(&mut gui);

    let gui_status = terminate_and_wait(&mut gui)?;
    let weston_status = terminate_and_wait(&mut weston)?;

    event_loop_result?;
    assert_terminated_cleanly(gui_status, "paddler_second_brain_gui")?;
    assert_terminated_cleanly(weston_status, "weston")?;

    Ok(())
}
