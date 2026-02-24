use std::cmp::Eq;
use std::cmp::Ordering;
use std::cmp::PartialEq;
use std::time::SystemTime;

use anyhow::anyhow;
use anyhow::Result;
use serde::Deserialize;
use serde::Serialize;

use crate::balancer::status_update::StatusUpdate;

#[derive(Clone, Debug, Eq, Serialize, Deserialize)]
pub struct UpstreamPeer {
    pub agent_id: String,
    pub model: Option<String>,
    pub last_update: SystemTime,
    pub quarantined_until: Option<SystemTime>,
    pub slots_taken: usize,
    pub slots_taken_since_last_status_update: usize,
    pub status: StatusUpdate,
}

impl UpstreamPeer {
    pub fn new_from_status_update(agent_id: String, status: StatusUpdate) -> Self {
        Self {
            agent_id,
            model: status.model.clone(),
            last_update: SystemTime::now(),
            quarantined_until: None,
            slots_taken: 0,
            slots_taken_since_last_status_update: 0,
            status,
        }
    }

    pub fn is_usable(&self) -> bool {
        !self.status.has_issues() && self.status.slots_idle > 0 && self.quarantined_until.is_none()
    }

    pub fn supports_model(&self, requested_model: &str) -> bool {
        requested_model.is_empty() || self.model.as_deref() == Some(requested_model)
    }

    pub fn is_usable_for_model(&self, requested_model: &str) -> bool {
        self.is_usable() && (requested_model.is_empty() || self.model.as_deref() == Some(requested_model))
    }

    pub fn release_slot(&mut self) -> Result<()> {
        if self.slots_taken < 1 {
            return Err(anyhow!(
                "Cannot release a slot when there are no taken slots"
            ));
        }

        self.last_update = SystemTime::now();
        self.slots_taken -= 1;

        if self.slots_taken_since_last_status_update > 0 {
            self.slots_taken_since_last_status_update -= 1;
            self.status.slots_idle += 1;
            self.status.slots_processing -= 1;
        }

        Ok(())
    }

    pub fn update_status(&mut self, status_update: StatusUpdate) {
        self.last_update = SystemTime::now();
        self.quarantined_until = None;
        self.slots_taken_since_last_status_update = 0;
        self.model = status_update.model.clone();
        self.status = status_update;
    }

    pub fn take_slot(&mut self) -> Result<()> {
        if self.status.slots_idle < 1 {
            return Err(anyhow!("Cannot take a slot when there are no idle slots"));
        }

        self.last_update = SystemTime::now();
        self.slots_taken_since_last_status_update += 1;
        self.slots_taken += 1;
        self.status.slots_idle -= 1;
        self.status.slots_processing += 1;

        Ok(())
    }
}

impl Ord for UpstreamPeer {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .is_usable()
            .cmp(&self.is_usable())
            .then_with(|| self.status.cmp(&other.status))
    }
}

impl PartialEq for UpstreamPeer {
    fn eq(&self, other: &Self) -> bool {
        self.agent_id == other.agent_id
    }
}

impl PartialOrd for UpstreamPeer {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[cfg(test)]
mod tests {
    use std::net::IpAddr;
    use std::net::Ipv4Addr;
    use std::net::SocketAddr;

    use super::*;
    use crate::llamacpp::slot::Slot;

    fn create_test_peer() -> UpstreamPeer {
        UpstreamPeer {
            agent_id: "test_agent".to_string(),
            model: Some("llama3".to_string()),
            last_update: SystemTime::now(),
            quarantined_until: None,
            slots_taken: 0,
            slots_taken_since_last_status_update: 0,
            status: StatusUpdate {
                agent_name: Some("test_name".to_string()),
                error: None,
                external_llamacpp_addr: SocketAddr::new(
                    IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                    8080,
                ),
                is_authorized: Some(true),
                is_connect_error: None,
                is_decode_error: None,
                is_deserialize_error: None,
                is_request_error: None,
                is_slots_endpoint_enabled: Some(true),
                is_unexpected_response_status: None,
                slots_idle: 5,
                slots_processing: 0,
                model: Some("llama3".to_string()),
            },
        }
    }

    #[test]
    fn test_take_release_slot() -> Result<()> {
        let mut peer = create_test_peer();

        assert_eq!(peer.status.slots_idle, 5);
        assert_eq!(peer.status.slots_processing, 0);

        peer.take_slot()?;

        assert_eq!(peer.status.slots_idle, 4);
        assert_eq!(peer.status.slots_processing, 1);

        peer.release_slot()?;

        assert_eq!(peer.status.slots_idle, 5);
        assert_eq!(peer.status.slots_processing, 0);

        Ok(())
    }

    #[test]
    fn test_take_slot_failure() {
        let mut peer = create_test_peer();

        peer.status.slots_idle = 0;

        assert!(peer.take_slot().is_err());
    }

    #[test]
    fn test_release_slot_failure() -> Result<()> {
        let mut peer = create_test_peer();

        peer.status.slots_processing = 0;

        assert!(peer.release_slot().is_err());

        Ok(())
    }

    #[test]
    fn test_update_status() {
        let mut peer = create_test_peer();
        let slots: Vec<Slot> = vec![];
        let slots_idle = slots.iter().filter(|slot| !slot.is_processing).count();

        let status_update = StatusUpdate {
            agent_name: Some("new_name".to_string()),
            error: None,
            external_llamacpp_addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8081),
            is_authorized: Some(true),
            is_connect_error: None,
            is_decode_error: None,
            is_deserialize_error: None,
            is_request_error: None,
            is_slots_endpoint_enabled: Some(true),
            is_unexpected_response_status: None,
            slots_idle,
            slots_processing: slots.len() - slots_idle,
            model: Some("llama3".to_string())
        };

        peer.update_status(status_update);
        assert_eq!(peer.status.slots_idle, 0);
        assert_eq!(peer.status.slots_processing, 0);
        assert_eq!(peer.status.agent_name, Some("new_name".to_string()));
    }
}
