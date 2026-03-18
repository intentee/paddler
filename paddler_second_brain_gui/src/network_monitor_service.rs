use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;
use paddler::service::Service;
use tokio::sync::broadcast;
use tokio::sync::mpsc;

use crate::detect_network_interfaces::detect_network_interfaces;
use crate::network_interface_address::NetworkInterfaceAddress;

pub struct NetworkMonitorService {
    pub network_interfaces_tx: mpsc::UnboundedSender<Vec<NetworkInterfaceAddress>>,
}

#[async_trait]
impl Service for NetworkMonitorService {
    fn name(&self) -> &'static str {
        "network_monitor"
    }

    async fn run(&mut self, mut shutdown_rx: broadcast::Receiver<()>) -> Result<()> {
        let mut interval = tokio::time::interval(Duration::from_secs(1));
        let mut previous_interfaces: Option<Vec<NetworkInterfaceAddress>> = None;

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    let interfaces = detect_network_interfaces();

                    let has_changed = previous_interfaces
                        .as_ref()
                        .map(|previous| *previous != interfaces)
                        .unwrap_or(true);

                    if has_changed {
                        if self.network_interfaces_tx.send(interfaces.clone()).is_err() {
                            log::warn!("Network interfaces receiver dropped");

                            break;
                        }

                        previous_interfaces = Some(interfaces);
                    }
                }
                _ = shutdown_rx.recv() => {
                    break;
                }
            }
        }

        Ok(())
    }
}
