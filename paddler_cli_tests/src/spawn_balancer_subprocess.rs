use std::ffi::OsStr;
use std::process::Stdio;

use anyhow::Context as _;
use anyhow::Result;
use futures_util::TryFutureExt as _;
use futures_util::future::ready;
use tokio::process::Child;

use paddler_test_cluster_harness::running_balancer::RunningBalancer;

use crate::paddler_command::paddler_command;
use crate::read_balancer_addresses::read_balancer_addresses;
use crate::signalled_subprocess::SignalledSubprocess;
use crate::spawned_balancer_subprocess::SpawnedBalancerSubprocess;
use crate::subprocess_cluster_error::SubprocessClusterError;

pub async fn spawn_balancer_subprocess<TArguments, TArgument, TSignalSubprocess>(
    binary_path: &str,
    arguments: TArguments,
    signal_subprocess: TSignalSubprocess,
) -> Result<SpawnedBalancerSubprocess>
where
    TArguments: IntoIterator<Item = TArgument>,
    TArgument: AsRef<OsStr>,
    TSignalSubprocess: FnOnce(Child) -> Result<SignalledSubprocess, SubprocessClusterError>,
{
    let mut balancer_subprocess = paddler_command(binary_path)
        .arg("balancer")
        .args(arguments)
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .context("failed to spawn paddler balancer subprocess")?;
    let Some(addresses) = ready(
        balancer_subprocess
            .stdout
            .take()
            .ok_or(SubprocessClusterError::StdoutNotPiped),
    )
    .and_then(read_balancer_addresses)
    .await?
    else {
        let exit_status = balancer_subprocess
            .wait()
            .await
            .map_err(SubprocessClusterError::ProcessExitUnobservable)?;

        return Err(SubprocessClusterError::BalancerExitedBeforeAnnouncing { exit_status }.into());
    };
    let SignalledSubprocess {
        process: balancer_process,
        signals,
    } = signal_subprocess(balancer_subprocess)?;

    RunningBalancer::new(addresses, Box::new(balancer_process))
        .map(|running_balancer| SpawnedBalancerSubprocess {
            running_balancer,
            signals,
        })
        .context("failed to address the balancer subprocess")
}
