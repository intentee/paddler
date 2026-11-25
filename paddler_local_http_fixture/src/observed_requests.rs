use std::io;
use std::io::ErrorKind;

use tokio::sync::watch;
use tokio::sync::watch::error::RecvError;

async fn wait_until_recorded<TRecord: Send + Sync>(
    records: &watch::Sender<Vec<TRecord>>,
    expected_count: usize,
) -> Result<(), RecvError> {
    records
        .subscribe()
        .wait_for(|recorded| recorded.len() >= expected_count)
        .await
        .map(drop)
}

pub struct ObservedRequests {
    abandoned_trickle_departures: watch::Sender<Vec<ErrorKind>>,
    range_headers: watch::Sender<Vec<Option<Vec<u8>>>>,
}

impl ObservedRequests {
    pub fn new() -> Self {
        Self {
            abandoned_trickle_departures: watch::Sender::new(Vec::new()),
            range_headers: watch::Sender::new(Vec::new()),
        }
    }

    pub fn abandoned_trickle_departures(&self) -> Vec<ErrorKind> {
        self.abandoned_trickle_departures.borrow().clone()
    }

    pub fn count(&self) -> usize {
        self.range_headers.borrow().len()
    }

    pub fn last_range_header(&self) -> Option<Vec<u8>> {
        self.range_headers.borrow().last().cloned().flatten()
    }

    pub fn record(&self, range_header: Option<Vec<u8>>) {
        self.range_headers
            .send_modify(|recorded_range_headers| recorded_range_headers.push(range_header));
    }

    pub fn record_abandoned_trickle(&self, client_departure: &io::Error) {
        self.abandoned_trickle_departures
            .send_modify(|departures| departures.push(client_departure.kind()));
    }

    pub async fn wait_for_abandoned_trickles(
        &self,
        expected_count: usize,
    ) -> Result<(), RecvError> {
        wait_until_recorded(&self.abandoned_trickle_departures, expected_count).await
    }

    pub async fn wait_for_count(&self, expected_count: usize) -> Result<(), RecvError> {
        wait_until_recorded(&self.range_headers, expected_count).await
    }
}
