use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

use dashmap::DashMap;
use dashmap::mapref::entry::Entry;
use nanoid::nanoid;
use tokio::sync::mpsc;

use crate::request_delivery::RequestDelivery;

pub struct RequestRegistry<TValue> {
    entries: DashMap<String, TValue>,
    fresh_request_id_prefix: String,
    next_fresh_request_id: AtomicU64,
}

impl<TValue> RequestRegistry<TValue> {
    pub fn insert_fresh(&self, value: TValue) -> String {
        let request_id = format!(
            "{}-{}",
            self.fresh_request_id_prefix,
            self.next_fresh_request_id.fetch_add(1, Ordering::Relaxed)
        );

        self.entries.insert(request_id.clone(), value);

        request_id
    }

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
            fresh_request_id_prefix: nanoid!(),
            next_fresh_request_id: AtomicU64::new(0),
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
    fn registers_every_fresh_value_under_a_distinct_request_id() {
        let registry = RequestRegistry::default();
        let first_request_id = registry.insert_fresh(1);
        let second_request_id = registry.insert_fresh(2);

        assert_ne!(first_request_id, second_request_id);
        assert_eq!(
            [
                registry.with_registered(&first_request_id, |value| *value),
                registry.with_registered(&second_request_id, |value| *value)
            ],
            [Some(1), Some(2)]
        );
    }

    #[test]
    fn two_registries_hand_out_distinct_fresh_request_ids() {
        assert_ne!(
            RequestRegistry::default().insert_fresh(1),
            RequestRegistry::default().insert_fresh(1)
        );
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
