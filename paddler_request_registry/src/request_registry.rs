use dashmap::DashMap;
use dashmap::mapref::entry::Entry;
use tokio::sync::mpsc;

use crate::request_delivery::RequestDelivery;

pub struct RequestRegistry<TValue> {
    entries: DashMap<String, TValue>,
}

impl<TValue> RequestRegistry<TValue> {
    #[must_use]
    pub fn insert_if_vacant(&self, request_id: String, value: TValue) -> bool {
        match self.entries.entry(request_id) {
            Entry::Occupied(_registered_request) => false,
            Entry::Vacant(vacant_request) => {
                vacant_request.insert(value);

                true
            }
        }
    }

    pub fn remove(&self, request_id: &str) {
        self.entries.remove(request_id);
    }

    pub fn with_registered<TOutput, TAction>(
        &self,
        request_id: &str,
        action: TAction,
    ) -> Option<TOutput>
    where
        TAction: FnOnce(&TValue) -> TOutput,
    {
        self.entries
            .get(request_id)
            .map(|registered_request| action(registered_request.value()))
    }
}

impl<TMessage> RequestRegistry<mpsc::UnboundedSender<TMessage>> {
    pub fn send_to(&self, request_id: &str, message: TMessage) -> RequestDelivery {
        match self.with_registered(request_id, |sender| sender.send(message)) {
            Some(Ok(())) => RequestDelivery::Delivered,
            Some(Err(_undelivered_message)) => RequestDelivery::ReceiverDropped,
            None => RequestDelivery::RequestNotRegistered,
        }
    }
}

impl<TValue> Default for RequestRegistry<TValue> {
    fn default() -> Self {
        Self {
            entries: DashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use tokio::sync::mpsc;

    use super::RequestRegistry;
    use crate::request_delivery::RequestDelivery;

    #[test]
    fn refuses_a_second_registration_under_the_same_request_id() {
        let registry = RequestRegistry::default();

        assert!(registry.insert_if_vacant("request".to_owned(), 1));
        assert!(!registry.insert_if_vacant("request".to_owned(), 2));
        assert_eq!(registry.with_registered("request", |value| *value), Some(1));
    }

    #[test]
    fn a_removed_request_is_no_longer_registered() {
        let registry = RequestRegistry::default();

        assert!(registry.insert_if_vacant("request".to_owned(), 1));

        registry.remove("request");

        assert_eq!(registry.with_registered("request", |value| *value), None);
    }

    #[test]
    fn sends_a_message_to_a_registered_request() {
        let registry = RequestRegistry::default();
        let (sender, mut receiver) = mpsc::unbounded_channel();

        assert!(registry.insert_if_vacant("request".to_owned(), sender));
        assert_eq!(registry.send_to("request", 7), RequestDelivery::Delivered);
        assert_eq!(receiver.try_recv(), Ok(7));
    }

    #[test]
    fn reports_a_request_whose_receiver_went_away() {
        let registry = RequestRegistry::default();
        let (sender, receiver) = mpsc::unbounded_channel();

        assert!(registry.insert_if_vacant("request".to_owned(), sender));

        drop(receiver);

        assert_eq!(
            registry.send_to("request", 7),
            RequestDelivery::ReceiverDropped
        );
    }

    #[test]
    fn reports_a_message_for_an_unknown_request() {
        let registry = RequestRegistry::<mpsc::UnboundedSender<i32>>::default();

        assert_eq!(
            registry.send_to("unknown", 7),
            RequestDelivery::RequestNotRegistered
        );
    }
}
