use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

use nix::errno::Errno;
use nix::sys::signal::Signal;
use nix::sys::signal::kill;
use nix::sys::wait::WaitPidFlag;
use nix::sys::wait::WaitStatus;
use nix::sys::wait::waitpid;
use nix::unistd::Pid;

use crate::subprocess_cluster_error::SubprocessClusterError;

#[derive(Clone, Debug)]
pub struct SubprocessSignals {
    pub kill_requested: Arc<AtomicBool>,
    pub pid: Pid,
}

impl SubprocessSignals {
    pub fn pause(&self) -> Result<(), SubprocessClusterError> {
        self.send(Signal::SIGSTOP)?;

        match waitpid(self.pid, Some(WaitPidFlag::WUNTRACED)) {
            Ok(WaitStatus::Stopped(..)) => Ok(()),
            Ok(status) => Err(SubprocessClusterError::ProcessDidNotStop {
                pid: self.pid,
                status,
            }),
            Err(source) => Err(SubprocessClusterError::ProcessUnobservable {
                pid: self.pid,
                source,
            }),
        }
    }

    pub fn resume(&self) -> Result<(), SubprocessClusterError> {
        self.send(Signal::SIGCONT)
    }

    pub fn kill(&self) -> Result<(), SubprocessClusterError> {
        self.kill_requested.store(true, Ordering::Release);

        self.send(Signal::SIGKILL)
    }

    pub fn terminate(&self) -> Result<(), SubprocessClusterError> {
        match self.send(Signal::SIGTERM) {
            Ok(())
            | Err(SubprocessClusterError::SignalUndeliverable {
                source: Errno::ESRCH,
                ..
            }) => Ok(()),
            Err(signal_error) => Err(signal_error),
        }
    }

    fn send(&self, signal: Signal) -> Result<(), SubprocessClusterError> {
        kill(self.pid, signal).map_err(|source| SubprocessClusterError::SignalUndeliverable {
            pid: self.pid,
            signal,
            source,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error as _;
    use std::io::BufRead as _;
    use std::io::BufReader;
    use std::process::Command;
    use std::process::Stdio;
    use std::sync::Arc;

    use nix::errno::Errno;
    use nix::sys::signal::Signal;
    use nix::sys::signal::kill;
    use nix::unistd::Pid;
    use nix::unistd::Uid;

    use super::SubprocessSignals;
    use crate::subprocess_cluster_error::SubprocessClusterError;

    fn signals_for(pid: Pid) -> SubprocessSignals {
        SubprocessSignals {
            kill_requested: Arc::default(),
            pid,
        }
    }

    #[test]
    fn terminating_a_process_this_user_may_not_signal_is_refused() {
        assert!(
            !Uid::effective().is_root(),
            "only an unprivileged user is refused when signalling the init process"
        );

        let terminate_error = signals_for(Pid::from_raw(1))
            .terminate()
            .expect_err("signalling the init process must be refused");

        assert_eq!(
            terminate_error.to_string(),
            SubprocessClusterError::SignalUndeliverable {
                pid: Pid::from_raw(1),
                signal: Signal::SIGTERM,
                source: Errno::EPERM,
            }
            .to_string()
        );
        assert_eq!(
            terminate_error
                .source()
                .and_then(|source| source.downcast_ref::<Errno>()),
            Some(&Errno::EPERM)
        );
    }

    #[test]
    fn pausing_a_process_this_harness_did_not_spawn_is_unobservable() {
        let mut shell = Command::new("sh")
            .args(["-c", "sleep 60 & echo $!; wait"])
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let mut grandchild_pid = String::new();

        BufReader::new(shell.stdout.take().unwrap())
            .read_line(&mut grandchild_pid)
            .unwrap();

        let grandchild = Pid::from_raw(grandchild_pid.trim_end().parse().unwrap());
        let pause_result = signals_for(grandchild).pause();

        kill(grandchild, Signal::SIGKILL).unwrap();
        shell.wait().unwrap();

        let pause_error =
            pause_result.expect_err("pausing a process this harness did not spawn must fail");

        assert_eq!(
            pause_error.to_string(),
            SubprocessClusterError::ProcessUnobservable {
                pid: grandchild,
                source: Errno::ECHILD,
            }
            .to_string()
        );
        assert_eq!(
            pause_error
                .source()
                .and_then(|source| source.downcast_ref::<Errno>()),
            Some(&Errno::ECHILD)
        );
    }
}
