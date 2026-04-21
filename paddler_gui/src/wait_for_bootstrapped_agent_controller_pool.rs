use std::sync::Arc;

use anyhow::Context;
use anyhow::Result;
use paddler::balancer::agent_controller_pool::AgentControllerPool;
use tokio::sync::watch;

pub async fn wait_for_bootstrapped_agent_controller_pool(
    watch_rx: &mut watch::Receiver<Option<Arc<AgentControllerPool>>>,
) -> Result<Arc<AgentControllerPool>> {
    loop {
        watch_rx.changed().await.context(
            "Bootstrap signal channel closed before publishing an agent controller pool",
        )?;

        let latest = watch_rx.borrow_and_update().clone();

        if let Some(pool) = latest {
            return Ok(pool);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use anyhow::Result;
    use anyhow::bail;
    use dashmap::DashMap;
    use paddler::balancer::agent_controller_pool::AgentControllerPool;
    use tokio::sync::Notify;
    use tokio::sync::watch;

    use super::wait_for_bootstrapped_agent_controller_pool;

    fn make_pool() -> Arc<AgentControllerPool> {
        Arc::new(AgentControllerPool {
            agents: DashMap::new(),
            update_notifier: Arc::new(Notify::new()),
        })
    }

    #[tokio::test]
    async fn returns_pool_once_sender_publishes_some() -> Result<()> {
        let pool = make_pool();
        let (watch_tx, mut watch_rx) = watch::channel::<Option<Arc<AgentControllerPool>>>(None);

        let publisher = {
            let pool = pool.clone();
            tokio::spawn(async move { watch_tx.send(Some(pool)) })
        };

        let received = wait_for_bootstrapped_agent_controller_pool(&mut watch_rx).await?;

        publisher.await??;

        if Arc::ptr_eq(&received, &pool) {
            Ok(())
        } else {
            bail!("helper returned a different Arc than was published")
        }
    }

    #[tokio::test]
    async fn returns_error_when_sender_is_dropped_before_any_some() -> Result<()> {
        let (watch_tx, mut watch_rx) = watch::channel::<Option<Arc<AgentControllerPool>>>(None);

        drop(watch_tx);

        match wait_for_bootstrapped_agent_controller_pool(&mut watch_rx).await {
            Err(_) => Ok(()),
            Ok(_) => bail!("helper returned Ok after sender was dropped"),
        }
    }

    #[tokio::test]
    async fn ignores_leading_none_values() -> Result<()> {
        let pool = make_pool();
        let (watch_tx, mut watch_rx) = watch::channel::<Option<Arc<AgentControllerPool>>>(None);

        let publisher = {
            let pool = pool.clone();
            tokio::spawn(async move {
                watch_tx.send(None)?;
                watch_tx.send(Some(pool))?;
                Ok::<(), watch::error::SendError<Option<Arc<AgentControllerPool>>>>(())
            })
        };

        let received = wait_for_bootstrapped_agent_controller_pool(&mut watch_rx).await?;

        publisher.await??;

        if Arc::ptr_eq(&received, &pool) {
            Ok(())
        } else {
            bail!("helper returned a different Arc than was published")
        }
    }
}
