use std::time::Duration;

use actix_web::Error;
use actix_web::Responder;
use actix_web::get;
use actix_web::web;
use actix_web_lab::sse;
use futures::StreamExt as _;

use crate::management_service::app_data::AppData;
use crate::serialize_snapshot_event::serialize_snapshot_event;
use crate::snapshots_stream::snapshots_stream;

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
