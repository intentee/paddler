use std::future::Future;
use std::panic::resume_unwind;
use std::sync::Arc;
use std::thread::Builder as ThreadBuilder;
use std::thread::JoinHandle;

use tokio::runtime::Builder as RuntimeBuilder;
use tokio::runtime::Handle;
use tokio_util::sync::CancellationToken;

use paddler_spawn_resuming_panic::spawn_blocking_resuming_panic::spawn_blocking_resuming_panic;

use crate::cluster_harness_error::ClusterHarnessError;
use crate::connection_keep_alive::ConnectionKeepAlive;

const CONNECTION_THREAD_NAME: &str = "cluster-connections";

pub struct ConnectionRuntime {
    connection_keep_alive: ConnectionKeepAlive,
    handle: Handle,
    thread: JoinHandle<()>,
}

impl ConnectionRuntime {
    /// # Errors
    ///
    /// Returns [`ClusterHarnessError::ConnectionRuntimeUnbuildable`] when the runtime cannot be built, and [`ClusterHarnessError::ConnectionThreadUnspawnable`] when the thread that drives it cannot be spawned.
    pub fn start() -> Result<Self, ClusterHarnessError> {
        let runtime = RuntimeBuilder::new_current_thread()
            .enable_all()
            .build()
            .map_err(ClusterHarnessError::ConnectionRuntimeUnbuildable)?;
        let handle = runtime.handle().clone();
        let stop = CancellationToken::new();
        let stop_requested = stop.clone();
        let thread = ThreadBuilder::new()
            .name(CONNECTION_THREAD_NAME.to_owned())
            .spawn(move || runtime.block_on(stop_requested.cancelled()))
            .map_err(ClusterHarnessError::ConnectionThreadUnspawnable)?;

        Ok(Self {
            connection_keep_alive: ConnectionKeepAlive {
                stop_guard: Arc::new(stop.drop_guard()),
            },
            handle,
            thread,
        })
    }

    #[must_use]
    pub fn keep_alive(&self) -> ConnectionKeepAlive {
        self.connection_keep_alive.clone()
    }

    /// # Errors
    ///
    /// Returns [`ClusterHarnessError::ConnectionTaskFailed`] when the work panics or the runtime stops before it completes.
    pub async fn run<TWork>(&self, work: TWork) -> Result<TWork::Output, ClusterHarnessError>
    where
        TWork: Future + Send + 'static,
        TWork::Output: Send + 'static,
    {
        self.handle
            .spawn(work)
            .await
            .map_err(ClusterHarnessError::ConnectionTaskFailed)
    }

    pub async fn shut_down(self) {
        let Self {
            connection_keep_alive,
            thread,
            ..
        } = self;

        drop(connection_keep_alive);

        spawn_blocking_resuming_panic(move || {
            thread
                .join()
                .unwrap_or_else(|panic_payload| resume_unwind(panic_payload));
        })
        .await;
    }
}
