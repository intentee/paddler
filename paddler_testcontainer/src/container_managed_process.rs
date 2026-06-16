use anyhow::Result;
use async_trait::async_trait;
use paddler_cluster::managed_process::ManagedProcess;
use testcontainers::ContainerAsync;
use testcontainers::GenericImage;

pub struct ContainerManagedProcess {
    container: ContainerAsync<GenericImage>,
}

impl ContainerManagedProcess {
    #[must_use]
    pub const fn new(container: ContainerAsync<GenericImage>) -> Self {
        Self { container }
    }
}

#[async_trait]
impl ManagedProcess for ContainerManagedProcess {
    async fn shutdown(&mut self) -> Result<()> {
        self.container.stop_with_timeout(Some(0)).await?;

        Ok(())
    }
}
