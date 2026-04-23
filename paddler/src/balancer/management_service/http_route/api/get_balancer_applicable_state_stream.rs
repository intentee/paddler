use std::convert::Infallible;
use std::time::Duration;

use actix_web::Error;
use actix_web::Responder;
use actix_web::get;
use actix_web::web;
use actix_web_lab::sse;
use log::error;

use crate::balancer::management_service::app_data::AppData;

pub fn register(cfg: &mut web::ServiceConfig) {
    cfg.service(respond);
}

#[get("/api/v1/balancer_applicable_state/stream")]
async fn respond(app_data: web::Data<AppData>) -> Result<impl Responder, Error> {
    let shutdown = app_data.shutdown.clone();
    let event_stream = async_stream::stream! {
        let serialize_state = |state| match serde_json::to_string(&state) {
            Ok(json) => Some(Ok::<_, Infallible>(sse::Event::Data(sse::Data::new(json)))),
            Err(err) => {
                error!("Failed to serialize balancer applicable state: {err}");
                None
            }
        };

        loop {
            let applicable_state = app_data
                .balancer_applicable_state_holder
                .get_agent_desired_state();

            if let Some(event) = serialize_state(applicable_state) {
                yield event;
            }

            tokio::select! {
                () = app_data.balancer_applicable_state_holder.update_notifier.notified() => {}
                () = shutdown.cancelled() => return,
            }
        }
    };

    Ok(sse::Sse::from_stream(event_stream).with_keep_alive(Duration::from_secs(10)))
}
