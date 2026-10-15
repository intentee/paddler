use std::io;
use std::io::ErrorKind;

use tokio::sync::watch;
use tokio::sync::watch::error::RecvError;

async fn wait_until_reached(
    counter: &watch::Sender<u32>,
    expected_count: u32,
) -> Result<(), RecvError> {
    counter
        .subscribe()
        .wait_for(|count| *count >= expected_count)
        .await
        .map(drop)
}

pub struct ObservedRequests {
    abandoned_trickle_departures: watch::Sender<Vec<ErrorKind>>,
    count: watch::Sender<u32>,
    last_range_header: watch::Sender<Option<Vec<u8>>>,
}

impl ObservedRequests {
    pub fn new() -> Self {
        Self {
            abandoned_trickle_departures: watch::Sender::new(Vec::new()),
            count: watch::Sender::new(0),
            last_range_header: watch::Sender::new(None),
        }
    }

    pub fn record(&self, range_header: Option<Vec<u8>>) {
        self.last_range_header.send_replace(range_header);
        self.count.send_modify(|count| *count += 1);
    }

    pub fn record_abandoned_trickle(&self, client_departure: &io::Error) {
        self.abandoned_trickle_departures
            .send_modify(|departures| departures.push(client_departure.kind()));
    }

    pub fn abandoned_trickle_departures(&self) -> Vec<ErrorKind> {
        self.abandoned_trickle_departures.borrow().clone()
    }

    pub fn count(&self) -> u32 {
        *self.count.borrow()
    }

    pub fn last_range_header(&self) -> Option<Vec<u8>> {
        self.last_range_header.borrow().clone()
    }

    pub async fn wait_for_count(&self, expected_count: u32) -> Result<(), RecvError> {
        wait_until_reached(&self.count, expected_count).await
    }

    pub async fn wait_for_abandoned_trickles(
        &self,
        expected_count: usize,
    ) -> Result<(), RecvError> {
        self.abandoned_trickle_departures
            .subscribe()
            .wait_for(|departures| departures.len() >= expected_count)
            .await
            .map(drop)
    }
}
