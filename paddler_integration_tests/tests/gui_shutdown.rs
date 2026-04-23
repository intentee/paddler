#![cfg(feature = "tests_that_use_compiled_paddler")]

use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;
use std::time::Instant;

use anyhow::Context as _;
use anyhow::Result;
use anyhow::anyhow;
use anyhow::bail;
use nix::sys::signal::Signal;
use nix::sys::signal::kill;
use nix::unistd::Pid;
use paddler_integration_tests::PADDLER_GUI_BINARY_PATH;
use tokio::io::AsyncBufReadExt as _;
use tokio::io::BufReader;
use tokio::process::Child;
use tokio::process::Command;

struct HeadlessDisplay {
    display_name: String,
    xvfb: Child,
}

impl HeadlessDisplay {
    async fn start() -> Result<Self> {
        let display_number = std::process::id() % 1000 + 99;
        let display_name = format!(":{display_number}");

        let mut xvfb = Command::new("Xvfb")
            .arg(&display_name)
            .arg("-screen")
            .arg("0")
            .arg("1024x768x24")
            .arg("-nolisten")
            .arg("tcp")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .context("failed to spawn Xvfb; is it installed in the nix-shell?")?;

        let lock_path = PathBuf::from(format!("/tmp/.X{display_number}-lock"));
        let socket_path = PathBuf::from(format!("/tmp/.X11-unix/X{display_number}"));

        loop {
            if lock_path.exists() && socket_path.exists() {
                break;
            }

            match xvfb.try_wait() {
                Ok(Some(exit_status)) => {
                    bail!("Xvfb exited before becoming ready: {exit_status}");
                }
                Ok(None) => {}
                Err(error) => bail!("failed to check Xvfb status: {error}"),
            }

            tokio::task::yield_now().await;
        }

        Ok(Self { display_name, xvfb })
    }

    fn display_name(&self) -> &str {
        &self.display_name
    }
}

impl Drop for HeadlessDisplay {
    fn drop(&mut self) {
        if let Some(raw_pid) = self.xvfb.id() {
            #[expect(clippy::cast_possible_wrap, reason = "PID values fit in i32")]
            let pid = Pid::from_raw(raw_pid as i32);
            let _ = kill(pid, Signal::SIGTERM);
        }
    }
}

async fn wait_for_log_line<TReader>(
    reader: &mut BufReader<TReader>,
    needle: &str,
    buffer: &mut Vec<String>,
) -> Result<()>
where
    TReader: tokio::io::AsyncRead + Unpin,
{
    let mut line = String::new();

    loop {
        line.clear();

        let bytes_read = reader
            .read_line(&mut line)
            .await
            .context("failed to read paddler_gui output")?;

        if bytes_read == 0 {
            bail!(
                "paddler_gui output ended before emitting {needle:?}; captured output:\n{}",
                buffer.join("")
            );
        }

        buffer.push(line.clone());

        if line.contains(needle) {
            return Ok(());
        }
    }
}

fn paddler_gui_binary_path() -> Result<PathBuf> {
    let path = PathBuf::from(PADDLER_GUI_BINARY_PATH.as_str());

    if !path.exists() {
        bail!(
            "paddler_gui binary not found at {}; build it first (e.g. `cargo build -p paddler_gui`)",
            path.display()
        );
    }

    Ok(path)
}

const SHUTDOWN_SLA: Duration = Duration::from_secs(1);

#[tokio::test]
async fn gui_binary_exits_under_one_second_on_sigterm_at_home_screen() -> Result<()> {
    let binary = paddler_gui_binary_path()?;
    let display = HeadlessDisplay::start().await?;

    let mut gui = Command::new(&binary)
        .env_remove("WAYLAND_DISPLAY")
        .env("DISPLAY", display.display_name())
        .env("RUST_LOG", "paddler_gui=info")
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .context("failed to spawn paddler_gui binary")?;

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

    let sigterm_sent_at = Instant::now();

    kill(pid, Signal::SIGTERM).context("failed to send SIGTERM to paddler_gui")?;

    let exit_status = gui
        .wait()
        .await
        .context("failed to wait for paddler_gui exit")?;

    let shutdown_elapsed = sigterm_sent_at.elapsed();

    assert!(
        exit_status.success() || exit_status.code().is_some(),
        "paddler_gui terminated abnormally: {exit_status:?}"
    );

    assert!(
        shutdown_elapsed < SHUTDOWN_SLA,
        "paddler_gui took {shutdown_elapsed:?} to exit after SIGTERM; SLA is {SHUTDOWN_SLA:?}"
    );

    drop(display);

    Ok(())
}
