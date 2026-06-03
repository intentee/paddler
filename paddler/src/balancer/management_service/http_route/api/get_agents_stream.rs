use std::convert::Infallible;
use std::time::Duration;

use actix_web::Error;
use actix_web::Responder;
use actix_web::get;
use actix_web::web;
use actix_web_lab::sse;
use futures::StreamExt as _;
use log::error;
use serde::Serialize;

use crate::balancer::management_service::app_data::AppData;
use crate::snapshots_stream::snapshots_stream;

fn serialize_snapshot_event<TSnapshot>(
    snapshot: &TSnapshot,
) -> Option<Result<sse::Event, Infallible>>
where
    TSnapshot: Serialize,
{
    match serde_json::to_string(snapshot) {
        Ok(json) => Some(Ok(sse::Event::Data(sse::Data::new(json)))),
        Err(err) => {
            error!("Failed to serialize agent controller pool snapshot: {err}");
            None
        }
    }
}

pub fn register(cfg: &mut web::ServiceConfig) {
    cfg.service(respond);
}

#[get("/api/v1/agents/stream")]
async fn respond(app_data: web::Data<AppData>) -> Result<impl Responder, Error> {
    let event_stream = snapshots_stream(
        app_data.agent_controller_pool.clone(),
        app_data.shutdown.clone(),
    )
    .filter_map(|snapshot| async move { serialize_snapshot_event(&snapshot) });

    Ok(sse::Sse::from_stream(event_stream).with_keep_alive(Duration::from_secs(10)))
}

#[cfg(test)]
mod tests {
    use serde::Serializer;
    use serde::ser::Error as _;

    use super::*;

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
    fn serialize_snapshot_event_returns_event_for_serializable_snapshot() {
        let event = serialize_snapshot_event(&"snapshot");

        assert!(event.is_some());
    }

    #[test]
    fn serialize_snapshot_event_skips_unserializable_snapshot() {
        log::set_max_level(log::LevelFilter::Trace);

        let event = serialize_snapshot_event(&FailingSnapshot);

        assert!(event.is_none());
    }
}
