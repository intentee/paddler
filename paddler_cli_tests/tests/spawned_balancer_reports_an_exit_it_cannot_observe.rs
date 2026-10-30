use std::error::Error as _;
use std::fs::OpenOptions;
use std::future::poll_fn;
use std::io;
use std::pin::pin;
use std::task::Poll;

use nix::errno::Errno;
use nix::sys::stat::Mode;
use nix::sys::wait::waitpid;
use nix::unistd::mkfifo;
use tempfile::TempDir;

use paddler_cli_tests::signalled_subprocess::SignalledSubprocess;
use paddler_cli_tests::spawn_balancer_subprocess::spawn_balancer_subprocess;
use paddler_cli_tests::subprocess_cluster_error::SubprocessClusterError;

#[tokio::test]
async fn spawned_balancer_reports_an_exit_it_cannot_observe() {
    let state_directory = TempDir::new().expect("the state directory must be created");
    let state_fifo_path = state_directory.path().join("state.fifo");

    mkfifo(&state_fifo_path, Mode::S_IRWXU).expect("the state fifo must be created");

    let mut balancer_spawn = pin!(spawn_balancer_subprocess(
        env!("CARGO_BIN_EXE_paddler_cluster_node"),
        [format!(
            "--state-database=file://{}",
            state_fifo_path.display()
        )],
        SignalledSubprocess::new,
    ));

    assert!(
        poll_fn(|context| Poll::Ready(balancer_spawn.as_mut().poll(context).is_pending())).await,
        "the balancer must wait for its state before it can announce"
    );

    drop(
        OpenOptions::new()
            .write(true)
            .open(&state_fifo_path)
            .expect("the balancer must open its state fifo"),
    );
    waitpid(None, None).expect("the balancer must exit and be reaped before the harness waits");

    let spawn_error = balancer_spawn
        .await
        .err()
        .expect("a balancer reaped elsewhere must not be observed exiting");

    assert_eq!(
        spawn_error.to_string(),
        SubprocessClusterError::ProcessExitUnobservable(io::Error::from(Errno::ECHILD)).to_string()
    );
    assert_eq!(
        spawn_error
            .downcast_ref::<SubprocessClusterError>()
            .and_then(|subprocess_cluster_error| subprocess_cluster_error.source())
            .and_then(|source| source.downcast_ref::<io::Error>())
            .and_then(io::Error::raw_os_error),
        Some(Errno::ECHILD as i32)
    );
}
