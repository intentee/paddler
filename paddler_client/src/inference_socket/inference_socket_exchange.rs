use std::collections::hash_map::Entry;
use std::mem;
use std::pin::pin;

use log::debug;
use parking_lot::Mutex;
use tokio::sync::Notify;
use tokio::sync::mpsc;
use tokio::sync::mpsc::UnboundedReceiver;

use paddler_messaging::inference_client::message::Message as InferenceMessage;

use crate::error::Error;
use crate::error::Result;
use crate::inference_socket::awaited_response::AwaitedResponse;
use crate::inference_socket::inference_socket_exchange_state::InferenceSocketExchangeState;
use crate::inference_socket::open_inference_socket_exchange::OpenInferenceSocketExchange;
use crate::inference_socket::outbound_frames::OutboundFrames;
use crate::inference_socket::outbound_request::OutboundRequest;

pub struct InferenceSocketExchange {
    closing: Notify,
    frames_queued: Notify,
    state: Mutex<InferenceSocketExchangeState>,
}

impl InferenceSocketExchange {
    pub fn close(&self) {
        let previous_state = mem::replace(
            &mut *self.state.lock(),
            InferenceSocketExchangeState::Closed,
        );

        self.closing.notify_waiters();
        self.frames_queued.notify_waiters();

        if let InferenceSocketExchangeState::Open(OpenInferenceSocketExchange {
            awaiting_responses,
            ..
        }) = previous_state
        {
            for (request_id, AwaitedResponse { response_tx, .. }) in awaiting_responses {
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

    pub async fn closed(&self) {
        let mut closing = pin!(self.closing.notified());

        closing.as_mut().enable();

        if !self.is_closed() {
            closing.await;
        }
    }

    pub fn deliver(&self, request_id: &str, message: InferenceMessage, is_done: bool) -> bool {
        if let InferenceSocketExchangeState::Open(open_exchange) = &mut *self.state.lock()
            && let Some(AwaitedResponse { response_tx, .. }) =
                open_exchange.awaiting_responses.get(request_id)
        {
            let send_failed = response_tx.send(Ok(message)).is_err();

            if is_done || send_failed {
                open_exchange.awaiting_responses.remove(request_id);
            }

            return true;
        }

        false
    }

    pub fn fail(&self, request_id: &str, error: Error) -> bool {
        if let InferenceSocketExchangeState::Open(open_exchange) = &mut *self.state.lock()
            && let Some(AwaitedResponse {
                response_tx,
                stop_frame,
            }) = open_exchange.awaiting_responses.remove(request_id)
        {
            if response_tx.send(Err(error)).is_err() {
                debug!("Receiver already dropped for request: {request_id}");
            }

            open_exchange.queued_frames.push(stop_frame);
            self.frames_queued.notify_waiters();

            return true;
        }

        false
    }

    #[must_use]
    pub fn is_closed(&self) -> bool {
        matches!(*self.state.lock(), InferenceSocketExchangeState::Closed)
    }

    pub async fn next_outbound_frames(&self) -> OutboundFrames {
        loop {
            let mut frames_queued = pin!(self.frames_queued.notified());

            frames_queued.as_mut().enable();

            match &mut *self.state.lock() {
                InferenceSocketExchangeState::Closed => return OutboundFrames::ExchangeClosed,
                InferenceSocketExchangeState::Open(open_exchange)
                    if !open_exchange.queued_frames.is_empty() =>
                {
                    return OutboundFrames::Queued(mem::take(&mut open_exchange.queued_frames));
                }
                InferenceSocketExchangeState::Open(_) => {}
            }

            frames_queued.await;
        }
    }

    pub fn register(
        &self,
        OutboundRequest {
            request_frame,
            request_id,
            stop_frame,
        }: OutboundRequest,
    ) -> Result<UnboundedReceiver<Result<InferenceMessage>>> {
        match &mut *self.state.lock() {
            InferenceSocketExchangeState::Closed => Err(Error::ConnectionDropped { request_id }),
            InferenceSocketExchangeState::Open(open_exchange) => {
                match open_exchange.awaiting_responses.entry(request_id) {
                    Entry::Occupied(in_flight_request) => Err(Error::InferenceRequestIdInFlight {
                        request_id: in_flight_request.key().clone(),
                    }),
                    Entry::Vacant(vacant_request) => {
                        let (response_tx, response_rx) = mpsc::unbounded_channel();

                        vacant_request.insert(AwaitedResponse {
                            response_tx,
                            stop_frame,
                        });
                        open_exchange.queued_frames.push(request_frame);
                        self.frames_queued.notify_waiters();

                        Ok(response_rx)
                    }
                }
            }
        }
    }

    pub fn stop_responding_to(&self, request_id: &str) {
        if let InferenceSocketExchangeState::Open(open_exchange) = &mut *self.state.lock()
            && let Some(AwaitedResponse { stop_frame, .. }) =
                open_exchange.awaiting_responses.remove(request_id)
        {
            open_exchange.queued_frames.push(stop_frame);
            self.frames_queued.notify_waiters();
        }
    }
}

impl Default for InferenceSocketExchange {
    fn default() -> Self {
        Self {
            closing: Notify::new(),
            frames_queued: Notify::new(),
            state: Mutex::new(InferenceSocketExchangeState::Open(
                OpenInferenceSocketExchange::default(),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use futures_util::FutureExt as _;
    use serde_json::json;
    use tokio::sync::mpsc::error::TryRecvError;

    use super::InferenceSocketExchange;
    use crate::error::Error;
    use crate::inference_socket::outbound_frames::OutboundFrames;
    use crate::inference_socket::outbound_request::OutboundRequest;

    fn outbound_request(request_id: &str) -> OutboundRequest {
        OutboundRequest::serialize(request_id.to_owned(), &json!({ "request": request_id }))
            .expect("a JSON request must serialize")
    }

    #[test]
    fn closing_notifies_open_requests_after_one_receiver_was_dropped() {
        let exchange = InferenceSocketExchange::default();
        let abandoned_request = exchange
            .register(outbound_request("abandoned"))
            .expect("an open exchange must accept a request");
        let mut awaited_request = exchange
            .register(outbound_request("awaited"))
            .expect("an open exchange must accept a request");

        drop(abandoned_request);
        exchange.close();

        assert!(matches!(
            awaited_request.try_recv(),
            Ok(Err(Error::ConnectionDropped { request_id })) if request_id == "awaited"
        ));
    }

    #[test]
    fn closing_an_already_closed_exchange_tells_each_request_only_once() {
        let exchange = InferenceSocketExchange::default();
        let mut awaited_request = exchange
            .register(outbound_request("awaited"))
            .expect("an open exchange must accept a request");

        exchange.close();
        exchange.close();

        assert!(matches!(
            awaited_request.try_recv(),
            Ok(Err(Error::ConnectionDropped { request_id })) if request_id == "awaited"
        ));
        assert_eq!(
            awaited_request.try_recv().err(),
            Some(TryRecvError::Disconnected)
        );
    }

    #[test]
    fn waiting_for_an_already_closed_exchange_resolves_at_once() {
        let exchange = InferenceSocketExchange::default();

        exchange.close();

        assert_eq!(exchange.closed().now_or_never(), Some(()));
    }

    #[test]
    fn a_closed_exchange_tells_its_writer_to_stop() {
        let exchange = InferenceSocketExchange::default();

        exchange.close();

        assert_eq!(
            exchange.next_outbound_frames().now_or_never(),
            Some(OutboundFrames::ExchangeClosed)
        );
    }

    #[test]
    fn stopping_an_awaited_request_queues_its_stop_frame_after_its_request_frame() {
        let exchange = InferenceSocketExchange::default();
        let stopped_request = outbound_request("stopped");
        let expected_frames = vec![
            stopped_request.request_frame.clone(),
            stopped_request.stop_frame.clone(),
        ];
        let stopped_response_rx = exchange
            .register(stopped_request)
            .expect("an open exchange must accept a request");

        exchange.stop_responding_to("stopped");
        drop(stopped_response_rx);

        assert_eq!(
            exchange.next_outbound_frames().now_or_never(),
            Some(OutboundFrames::Queued(expected_frames))
        );
    }

    #[test]
    fn failing_a_request_queues_its_stop_frame_after_its_request_frame() {
        let exchange = InferenceSocketExchange::default();
        let failed_request = outbound_request("failed");
        let expected_frames = vec![
            failed_request.request_frame.clone(),
            failed_request.stop_frame.clone(),
        ];
        let _failed_response_rx = exchange
            .register(failed_request)
            .expect("an open exchange must accept a request");

        exchange.fail(
            "failed",
            Error::InferenceRequestCancelled {
                request_id: "failed".to_owned(),
            },
        );

        assert_eq!(
            exchange.next_outbound_frames().now_or_never(),
            Some(OutboundFrames::Queued(expected_frames))
        );
    }

    #[test]
    fn failing_a_request_that_is_not_pending_fails_nothing() {
        let exchange = InferenceSocketExchange::default();

        assert!(!exchange.fail(
            "unknown",
            Error::InferenceRequestCancelled {
                request_id: "unknown".to_owned(),
            },
        ));
    }

    #[test]
    fn failing_an_abandoned_request_forgets_it() {
        let exchange = InferenceSocketExchange::default();
        let abandoned_request = exchange
            .register(outbound_request("abandoned"))
            .expect("an open exchange must accept a request");

        drop(abandoned_request);

        assert!(exchange.fail(
            "abandoned",
            Error::InferenceRequestCancelled {
                request_id: "abandoned".to_owned(),
            },
        ));
        assert!(!exchange.fail(
            "abandoned",
            Error::InferenceRequestCancelled {
                request_id: "abandoned".to_owned(),
            },
        ));
    }

    #[test]
    fn a_second_registration_of_an_in_flight_request_keeps_the_first_one() {
        let exchange = InferenceSocketExchange::default();
        let mut first_registration = exchange
            .register(outbound_request("request-1"))
            .expect("an open exchange must accept a request");
        assert!(matches!(
            exchange.register(outbound_request("request-1")),
            Err(Error::InferenceRequestIdInFlight { request_id }) if request_id == "request-1"
        ));

        exchange.close();

        assert!(matches!(
            first_registration.try_recv(),
            Ok(Err(Error::ConnectionDropped { request_id })) if request_id == "request-1"
        ));
    }
}
