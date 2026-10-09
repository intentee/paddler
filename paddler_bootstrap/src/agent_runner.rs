use crate::agent_runner_params::AgentRunnerParams;
use crate::agent_service_bundle::AgentServiceBundle;
use crate::bootstrap_error::BootstrapError;
use crate::run_service_manager::run_service_manager;
use crate::service_thread::ServiceThread;

pub struct AgentRunner {
    thread: ServiceThread,
}

impl AgentRunner {
    #[must_use]
    pub fn start(
        AgentRunnerParams {
            bootstrap_config,
            cancellation_token,
            shutdown_options,
        }: AgentRunnerParams,
    ) -> Self {
        let bundle = AgentServiceBundle::new(bootstrap_config);

        let thread = ServiceThread::spawn(cancellation_token, move |task_shutdown| {
            run_service_manager(bundle, task_shutdown, shutdown_options)
        });

        Self { thread }
    }

    pub async fn wait_for_completion(&mut self) -> Result<(), BootstrapError> {
        self.thread.wait_for_completion().await
    }

    pub fn cancel(&self) {
        self.thread.cancel();
    }
}
