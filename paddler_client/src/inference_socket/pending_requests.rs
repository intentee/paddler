use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::mem;

use log::debug;
use parking_lot::Mutex;
use tokio::sync::mpsc;
use tokio::sync::mpsc::UnboundedReceiver;

use paddler_messaging::inference_client::message::Message as InferenceMessage;

use crate::error::Error;
use crate::error::Result;
use crate::inference_socket::pending_requests_state::PendingRequestsState;

pub struct PendingRequests {
    state: Mutex<PendingRequestsState>,
}

impl PendingRequests {
    pub fn close(&self) {
        let previous_state = mem::replace(&mut *self.state.lock(), PendingRequestsState::Closed);

        if let PendingRequestsState::Open(requests) = previous_state {
            for (request_id, response_tx) in requests {
                if response_tx
                    .send(Err(Error::ConnectionDropped {
                        request_id: request_id.clone(),
                    }))
                    .is_err()
                {
                    debug!("Receiver already dropped for request: {request_id}");
                }
            }
        }
    }

    pub fn deliver(&self, request_id: &str, message: InferenceMessage, is_done: bool) -> bool {
        if let PendingRequestsState::Open(requests) = &mut *self.state.lock()
            && let Some(response_tx) = requests.get(request_id)
        {
            let send_failed = response_tx.send(Ok(message)).is_err();

            if is_done || send_failed {
                requests.remove(request_id);
            }

            return true;
        }

        false
    }

    pub fn fail(&self, request_id: &str, error: Error) -> bool {
        if let PendingRequestsState::Open(requests) = &mut *self.state.lock()
            && let Some(response_tx) = requests.remove(request_id)
        {
            if response_tx.send(Err(error)).is_err() {
                debug!("Receiver already dropped for request: {request_id}");
            }

            return true;
        }

        false
    }

    #[must_use]
    pub fn is_closed(&self) -> bool {
        matches!(*self.state.lock(), PendingRequestsState::Closed)
    }

    pub fn register(
        &self,
        request_id: String,
    ) -> Result<UnboundedReceiver<Result<InferenceMessage>>> {
        match &mut *self.state.lock() {
            PendingRequestsState::Closed => Err(Error::ConnectionDropped { request_id }),
            PendingRequestsState::Open(requests) => match requests.entry(request_id) {
                Entry::Occupied(in_flight_request) => Err(Error::InferenceRequestIdInFlight {
                    request_id: in_flight_request.key().clone(),
                }),
                Entry::Vacant(vacant_request) => {
                    let (response_tx, response_rx) = mpsc::unbounded_channel();

                    vacant_request.insert(response_tx);

                    Ok(response_rx)
                }
            },
        }
    }

    pub fn remove(&self, request_id: &str) {
        if let PendingRequestsState::Open(requests) = &mut *self.state.lock() {
            requests.remove(request_id);
        }
    }
}

impl Default for PendingRequests {
    fn default() -> Self {
        Self {
            state: Mutex::new(PendingRequestsState::Open(HashMap::new())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::PendingRequests;
    use crate::error::Error;

    #[test]
    fn closing_notifies_open_requests_after_one_receiver_was_dropped() {
        let pending_requests = PendingRequests::default();
        let abandoned_request = pending_requests
            .register("abandoned".to_owned())
            .expect("an open registry must accept a request");
        let mut awaited_request = pending_requests
            .register("awaited".to_owned())
            .expect("an open registry must accept a request");

        drop(abandoned_request);
        pending_requests.close();

        assert!(matches!(
            awaited_request.try_recv(),
            Ok(Err(Error::ConnectionDropped { request_id })) if request_id == "awaited"
        ));
    }

    #[test]
    fn failing_a_request_that_is_not_pending_fails_nothing() {
        let pending_requests = PendingRequests::default();

        assert!(!pending_requests.fail(
            "unknown",
            Error::InferenceRequestCancelled {
                request_id: "unknown".to_owned(),
            },
        ));
    }

    #[test]
    fn failing_an_abandoned_request_forgets_it() {
        let pending_requests = PendingRequests::default();
        let abandoned_request = pending_requests
            .register("abandoned".to_owned())
            .expect("an open registry must accept a request");

        drop(abandoned_request);

        assert!(pending_requests.fail(
            "abandoned",
            Error::InferenceRequestCancelled {
                request_id: "abandoned".to_owned(),
            },
        ));
        assert!(!pending_requests.fail(
            "abandoned",
            Error::InferenceRequestCancelled {
                request_id: "abandoned".to_owned(),
            },
        ));
    }

    #[test]
    fn a_second_registration_of_an_in_flight_request_keeps_the_first_one() {
        let pending_requests = PendingRequests::default();
        let mut first_registration = pending_requests
            .register("request-1".to_owned())
            .expect("an open registry must accept a request");
        assert!(matches!(
            pending_requests.register("request-1".to_owned()),
            Err(Error::InferenceRequestIdInFlight { request_id }) if request_id == "request-1"
        ));

        pending_requests.close();

        assert!(matches!(
            first_registration.try_recv(),
            Ok(Err(Error::ConnectionDropped { request_id })) if request_id == "request-1"
        ));
    }
}
