use std::process::ExitStatus;

use anyhow::Result;
use tokio::process::Child;

use crate::send_sigterm_if_running::send_sigterm_if_running;

pub async fn terminate_subprocess(mut child: Child) -> Result<ExitStatus> {
    send_sigterm_if_running(&child)?;

    let exit_status = child.wait().await?;

    Ok(exit_status)
}
