use tokio::process::Child;

use crate::subprocess_cluster_error::SubprocessClusterError;
use crate::subprocess_process::SubprocessProcess;
use crate::subprocess_signals::SubprocessSignals;

pub struct SignalledSubprocess {
    pub process: SubprocessProcess,
    pub signals: SubprocessSignals,
}

impl SignalledSubprocess {
    pub fn new(child: Child) -> Result<Self, SubprocessClusterError> {
        let process = SubprocessProcess::new(child);
        let signals = process
            .signals()
            .ok_or(SubprocessClusterError::ProcessAlreadyReaped)?;

        Ok(Self { process, signals })
    }
}
