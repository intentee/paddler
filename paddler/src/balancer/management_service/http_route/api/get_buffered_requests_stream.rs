use std::convert::Infallible;
use std::time::Duration;

use actix_web::Error;
use actix_web::Responder;
use actix_web::get;
use actix_web::web;
use actix_web_lab::sse;
use log::error;

use crate::balancer::management_service::app_data::AppData;
use crate::produces_snapshot::ProducesSnapshot as _;

pub fn register(cfg: &mut web::ServiceConfig) {
    cfg.service(respond);
}

#[get("/api/v1/buffered_requests/stream")]
async fn respond(app_data: web::Data<AppData>) -> Result<impl Responder, Error> {
    let event_stream = async_stream::stream! {
        let send_event = |info| {
            match serde_json::to_string(&info) {
                Ok(json) => Some(Ok::<_, Infallible>(sse::Event::Data(sse::Data::new(json)))),
                Err(err) => {
                    error!("Failed to serialize buffered requests info: {err}");
                    None
                }
            }
        };

        loop {
            match app_data.buffered_request_manager.make_snapshot() {
                Ok(buffered_request_manager_snapshot) => {
                    if let Some(event) = send_event(buffered_request_manager_snapshot) {
                        yield event;
                    }
                },
                Err(err) => error!("Failed to get buffered requests snapshot: {err}"),
            }

            app_data.buffered_request_manager.update_notifier.notified().await;
        }
    };

    Ok(sse::Sse::from_stream(event_stream).with_keep_alive(Duration::from_secs(10)))
}
