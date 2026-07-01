use anyhow::Result;
use anyhow::anyhow;
use dashmap::DashMap;

use crate::llamacpp_instance::LlamaCppInstance;

#[derive(Debug, Default)]
pub struct LlamaCppInstanceCollection {
    pub completion_response_delay: Option<i64>,
    pub instances: DashMap<String, LlamaCppInstance>,
    pub last_llamacpp_port_offset: u16,
}

impl LlamaCppInstanceCollection {
    pub async fn cleanup(&mut self) {
        for mut llama in self.instances.iter_mut() {
            llama.cleanup().await;
        }

        self.instances.clear();
        self.last_llamacpp_port_offset = 0;
    }

    pub fn llamacpp_port(&self, llamacpp_name: &str) -> Result<u16> {
        if let Some(llama) = self.instances.get(llamacpp_name) {
            Ok(llama.port)
        } else {
            Err(anyhow!("LlamaCpp instance {} not found", llamacpp_name))
        }
    }

    pub fn next_llamacpp_port(&mut self) -> u16 {
        let port = 8000 + self.last_llamacpp_port_offset;

        self.last_llamacpp_port_offset += 1;

        port
    }

    pub async fn kill(&self, llamacpp_name: &str) -> Result<()> {
        if let Some((_, mut llamacpp)) = self.instances.remove(llamacpp_name) {
            llamacpp.cleanup().await;
            Ok(())
        } else {
            Err(anyhow!("LlamaCpp instance {} not found", llamacpp_name))
        }
    }
}
