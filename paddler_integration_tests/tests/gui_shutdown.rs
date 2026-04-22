#![cfg(feature = "tests_that_use_compiled_paddler")]

use std::path::PathBuf;
use std::process::Stdio;

use anyhow::Context as _;
use anyhow::Result;
use anyhow::anyhow;
use anyhow::bail;
use nix::sys::signal::Signal;
use nix::sys::signal::kill;
use nix::unistd::Pid;
use paddler_integration_tests::PADDLER_GUI_BINARY_PATH;
use tempfile::TempDir;
use tokio::io::AsyncBufReadExt as _;
use tokio::io::BufReader;
use tokio::process::Child;
use tokio::process::Command;

struct HeadlessWayland {
    socket_name: String,
    runtime_dir: TempDir,
    weston: Child,
}

impl HeadlessWayland {
    async fn start() -> Result<Self> {
        let runtime_dir = tempfile::Builder::new()
            .prefix("paddler_gui_test_xdg_")
            .tempdir()
            .context("failed to create runtime dir for headless wayland")?;
        let socket_name = format!("paddler-gui-test-{}", std::process::id());

        let mut weston = Command::new("weston")
            .arg("--backend=headless")
            .arg(format!("--socket={socket_name}"))
            .arg("--width=800")
            .arg("--height=800")
            .env("XDG_RUNTIME_DIR", runtime_dir.path())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .context("failed to spawn weston; is it installed in the nix-shell?")?;

        let socket_path = runtime_dir.path().join(&socket_name);

        loop {
            if socket_path.exists() {
                break;
            }

            match weston.try_wait() {
                Ok(Some(exit_status)) => {
                    bail!("weston exited before creating socket: {exit_status}");
                }
                Ok(None) => {}
                Err(error) => bail!("failed to check weston status: {error}"),
            }

            tokio::task::yield_now().await;
        }

        Ok(Self {
            socket_name,
            runtime_dir,
            weston,
        })
    }

    fn runtime_dir(&self) -> &std::path::Path {
        self.runtime_dir.path()
    }

    fn socket_name(&self) -> &str {
        &self.socket_name
    }
}

impl Drop for HeadlessWayland {
    fn drop(&mut self) {
        if let Some(raw_pid) = self.weston.id() {
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

#[tokio::test]
async fn gui_binary_exits_cleanly_on_sigterm_at_home_screen() -> Result<()> {
    let binary = paddler_gui_binary_path()?;
    let wayland = HeadlessWayland::start().await?;

    let mut gui = Command::new(&binary)
        .env("XDG_RUNTIME_DIR", wayland.runtime_dir())
        .env("WAYLAND_DISPLAY", wayland.socket_name())
        .env("RUST_LOG", "info")
        .stdout(Stdio::piped())
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

    kill(pid, Signal::SIGTERM).context("failed to send SIGTERM to paddler_gui")?;

    let exit_status = gui
        .wait()
        .await
        .context("failed to wait for paddler_gui exit")?;

    assert!(
        exit_status.success() || exit_status.code().is_some(),
        "paddler_gui terminated abnormally: {exit_status:?}"
    );

    drop(wayland);

    Ok(())
}
