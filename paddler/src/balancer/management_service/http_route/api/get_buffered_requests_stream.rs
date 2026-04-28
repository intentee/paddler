use std::convert::Infallible;
use std::time::Duration;

use actix_web::Error;
use actix_web::Responder;
use actix_web::get;
use actix_web::web;
use actix_web_lab::sse;
use futures::StreamExt as _;
use log::error;

use crate::balancer::management_service::app_data::AppData;
use crate::snapshots_stream::snapshots_stream;

pub fn register(cfg: &mut web::ServiceConfig) {
    cfg.service(respond);
}

#[get("/api/v1/buffered_requests/stream")]
async fn respond(app_data: web::Data<AppData>) -> Result<impl Responder, Error> {
    let event_stream = snapshots_stream(
        app_data.buffered_request_manager.clone(),
        app_data.shutdown.clone(),
    )
    .filter_map(|snapshot| async move {
        match serde_json::to_string(&snapshot) {
            Ok(json) => Some(Ok::<_, Infallible>(sse::Event::Data(sse::Data::new(json)))),
            Err(err) => {
                error!("Failed to serialize buffered requests snapshot: {err}");
                None
            }
        }
    });

    Ok(sse::Sse::from_stream(event_stream).with_keep_alive(Duration::from_secs(10)))
}
