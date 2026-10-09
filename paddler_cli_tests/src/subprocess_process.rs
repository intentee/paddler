use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

use anyhow::Result;
use async_trait::async_trait;
use log::warn;
use nix::errno::Errno;
use nix::unistd::Pid;
use tokio::process::Child;

use paddler_test_cluster_harness::managed_process::ManagedProcess;
use paddler_test_cluster_harness::process_end::ProcessEnd;

use crate::subprocess_cluster_error::SubprocessClusterError;
use crate::subprocess_signals::SubprocessSignals;

pub struct SubprocessProcess {
    child: Child,
    kill_requested: Arc<AtomicBool>,
}

impl SubprocessProcess {
    #[must_use]
    pub fn new(child: Child) -> Self {
        Self {
            child,
            kill_requested: Arc::default(),
        }
    }

    #[must_use]
    pub fn signals(&self) -> Option<SubprocessSignals> {
        self.child.id().map(|raw_pid| SubprocessSignals {
            kill_requested: self.kill_requested.clone(),
            pid: Pid::from_raw(raw_pid.cast_signed()),
        })
    }
}

#[async_trait]
impl ManagedProcess for SubprocessProcess {
    fn terminate(&mut self) -> Result<()> {
        let Some(signals) = self.signals() else {
            return Ok(());
        };

        match signals.terminate() {
            Ok(())
            | Err(SubprocessClusterError::SignalUndeliverable {
                source: Errno::ESRCH,
                ..
            }) => Ok(()),
            Err(signal_error) => Err(signal_error.into()),
        }
    }

    async fn exited(&mut self) -> ProcessEnd {
        match self.child.wait().await {
            Ok(exit_status)
                if exit_status.success() || self.kill_requested.load(Ordering::Acquire) =>
            {
                ProcessEnd::Clean
            }
            Ok(exit_status) => {
                ProcessEnd::failed(SubprocessClusterError::ProcessExitedWithFailure { exit_status })
            }
            Err(source) => {
                ProcessEnd::failed(SubprocessClusterError::ProcessExitUnobservable(source))
            }
        }
    }
}

impl Drop for SubprocessProcess {
    fn drop(&mut self) {
        if let Err(error) = self.terminate() {
            warn!("SubprocessProcess drop: failed to terminate subprocess: {error:#}");
        }
    }
}
