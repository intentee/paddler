use std::convert::Infallible;
use std::time::Duration;

use actix_web::Error;
use actix_web::Responder;
use actix_web::error::ErrorInternalServerError;
use actix_web::get;
use actix_web::web;
use actix_web_lab::sse;
use log::error;
use tokio::sync::broadcast::error::RecvError;

use crate::balancer::management_service::app_data::AppData;

pub fn register(cfg: &mut web::ServiceConfig) {
    cfg.service(respond);
}

#[get("/api/v1/balancer_desired_state/stream")]
async fn respond(app_data: web::Data<AppData>) -> Result<impl Responder, Error> {
    let shutdown = app_data.shutdown.clone();
    let mut receiver = app_data.balancer_desired_state_tx.subscribe();
    let initial_state = app_data
        .state_database
        .read_balancer_desired_state()
        .await
        .map_err(ErrorInternalServerError)?;

    let event_stream = async_stream::stream! {
        let serialize_state = |state| match serde_json::to_string(&state) {
            Ok(json) => Some(Ok::<_, Infallible>(sse::Event::Data(sse::Data::new(json)))),
            Err(err) => {
                error!("Failed to serialize balancer desired state: {err}");
                None
            }
        };

        if let Some(event) = serialize_state(initial_state) {
            yield event;
        }

        loop {
            tokio::select! {
                biased;
                () = shutdown.cancelled() => return,
                recv_result = receiver.recv() => {
                    match recv_result {
                        Ok(state) => {
                            if let Some(event) = serialize_state(state) {
                                yield event;
                            }
                        }
                        Err(RecvError::Lagged(_)) => {}
                        Err(RecvError::Closed) => return,
                    }
                }
            }
        }
    };

    Ok(sse::Sse::from_stream(event_stream).with_keep_alive(Duration::from_secs(10)))
}
