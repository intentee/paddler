use std::convert::Infallible;

use actix_web_lab::sse;
use log::error;
use serde::Serialize;

pub fn serialize_snapshot_event<TSnapshot>(
    snapshot: &TSnapshot,
) -> Option<Result<sse::Event, Infallible>>
where
    TSnapshot: Serialize,
{
    match serde_json::to_string(snapshot) {
        Ok(json) => Some(Ok(sse::Event::Data(sse::Data::new(json)))),
        Err(err) => {
            error!("Failed to serialize snapshot: {err}");

            None
        }
    }
}

#[cfg(test)]
mod tests {
    use serde::Serialize;
    use serde::Serializer;
    use serde::ser::Error as _;

    use super::serialize_snapshot_event;

    struct FailingSnapshot;

    impl Serialize for FailingSnapshot {
        fn serialize<TSerializer>(
            &self,
            _serializer: TSerializer,
        ) -> Result<TSerializer::Ok, TSerializer::Error>
        where
            TSerializer: Serializer,
        {
            Err(TSerializer::Error::custom("snapshot cannot be serialized"))
        }
    }

    #[test]
    fn returns_event_for_serializable_snapshot() {
        let event = serialize_snapshot_event(&"snapshot");

        assert!(event.is_some());
    }

    #[test]
    fn skips_unserializable_snapshot() {
        log::set_max_level(log::LevelFilter::Trace);

        let event = serialize_snapshot_event(&FailingSnapshot);

        assert!(event.is_none());
    }
}
