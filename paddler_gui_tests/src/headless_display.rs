use std::path::PathBuf;
use std::process::Stdio;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering;
use std::time::Duration;

use anyhow::Context as _;
use anyhow::Result;
use anyhow::bail;
use paddler_tests::terminate_child::terminate_child;
use tokio::process::Child;
use tokio::process::Command;

const XVFB_READINESS_PROBE_INTERVAL: Duration = Duration::from_millis(20);

static NEXT_DISPLAY_OFFSET: AtomicU32 = AtomicU32::new(0);

pub struct HeadlessDisplay {
    display_name: String,
    xvfb: Child,
}

impl HeadlessDisplay {
    pub async fn start() -> Result<Self> {
        let base = std::process::id() % 800 + 99;
        let offset = NEXT_DISPLAY_OFFSET.fetch_add(1, Ordering::Relaxed);
        let display_number = base + offset;
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

            tokio::time::sleep(XVFB_READINESS_PROBE_INTERVAL).await;
        }

        Ok(Self { display_name, xvfb })
    }

    #[must_use]
    pub fn display_name(&self) -> &str {
        &self.display_name
    }
}

impl Drop for HeadlessDisplay {
    fn drop(&mut self) {
        let _ = terminate_child(&mut self.xvfb);
    }
}
