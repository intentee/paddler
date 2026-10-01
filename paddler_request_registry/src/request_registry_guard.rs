use std::sync::Arc;

use crate::request_registry::RequestRegistry;

pub struct RequestRegistryGuard<TValue> {
    pub request_id: String,
    registry: Arc<RequestRegistry<TValue>>,
}

impl<TValue> RequestRegistryGuard<TValue> {
    #[must_use]
    pub fn register(
        registry: &Arc<RequestRegistry<TValue>>,
        request_id: String,
        value: TValue,
    ) -> Option<Self> {
        registry
            .insert_if_vacant(request_id.clone(), value)
            .then(|| Self {
                request_id,
                registry: registry.clone(),
            })
    }
}

impl<TValue> Drop for RequestRegistryGuard<TValue> {
    fn drop(&mut self) {
        self.registry.remove(&self.request_id);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::RequestRegistryGuard;
    use crate::request_registry::RequestRegistry;

    #[test]
    fn dropping_the_guard_frees_its_request_id() {
        let registry = Arc::new(RequestRegistry::default());
        let guard = RequestRegistryGuard::register(&registry, "request".to_owned(), 1);

        assert_eq!(registry.with_registered("request", |value| *value), Some(1));
        assert!(RequestRegistryGuard::register(&registry, "request".to_owned(), 2).is_none());

        drop(guard);

        assert_eq!(registry.with_registered("request", |value| *value), None);
    }
}
