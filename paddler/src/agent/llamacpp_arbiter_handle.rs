use std::thread;

use actix::Addr;
use anyhow::Result;
use anyhow::anyhow;
use tokio::sync::oneshot;

use crate::agent::llamacpp_slot::LlamaCppSlot;

pub struct LlamaCppArbiterHandle {
    pub llamacpp_slot_addr: Addr<LlamaCppSlot>,
    pub shutdown_tx: oneshot::Sender<()>,
    pub sync_arbiter_thread_handle: thread::JoinHandle<Result<()>>,
}

impl LlamaCppArbiterHandle {
    pub fn shutdown(self) -> Result<()> {
        self.shutdown_tx
            .send(())
            .map_err(|err| anyhow!("Failed to send shutdown signal: {err:?}"))?;

        self.sync_arbiter_thread_handle
            .join()
            .map_err(|err| anyhow!("Failed to join sync arbiter thread: {err:?}"))??;

        Ok(())
    }
}
