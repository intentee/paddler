use anyhow::Result;
use async_trait::async_trait;
use log::warn;
use tokio::process::Child;

use paddler_cluster_harness::managed_process::ManagedProcess;

use crate::terminate_child::terminate_child;

pub struct SubprocessProcess {
    child: Child,
}

impl SubprocessProcess {
    #[must_use]
    pub const fn new(child: Child) -> Self {
        Self { child }
    }
}

#[async_trait]
impl ManagedProcess for SubprocessProcess {
    async fn shutdown(&mut self) -> Result<()> {
        terminate_child(&mut self.child)?;
        self.child.wait().await?;

        Ok(())
    }
}

impl Drop for SubprocessProcess {
    fn drop(&mut self) {
        if let Err(error) = terminate_child(&mut self.child) {
            warn!("SubprocessProcess drop: failed to terminate subprocess: {error:#}");
        }
    }
}
