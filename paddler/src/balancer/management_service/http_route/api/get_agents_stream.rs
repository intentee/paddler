use std::convert::Infallible;
use std::time::Duration;

use actix_web::Error;
use actix_web::Responder;
use actix_web::get;
use actix_web::web;
use actix_web_lab::sse;
use log::error;
use paddler_types::agent_controller_pool_snapshot::AgentControllerPoolSnapshot;

use super::get_agents::mock_agents;
use crate::balancer::management_service::app_data::AppData;

pub fn register(cfg: &mut web::ServiceConfig) {
    cfg.service(respond);
}

#[get("/api/v1/agents/stream")]
async fn respond(app_data: web::Data<AppData>) -> Result<impl Responder, Error> {
    let event_stream = async_stream::stream! {
        let send_event = |info| {
            match serde_json::to_string(&info) {
                Ok(json) => Some(Ok::<_, Infallible>(sse::Event::Data(sse::Data::new(json)))),
                Err(err) => {
                    error!("Failed to serialize pool info: {err}");
                    None
                }
            }
        };

        loop {
            let snapshot = AgentControllerPoolSnapshot {
                agents: mock_agents(),
            };

            if let Some(event) = send_event(snapshot) {
                yield event;
            }

            app_data.agent_controller_pool.update_notifier.notified().await;
        }
    };

    Ok(sse::Sse::from_stream(event_stream).with_keep_alive(Duration::from_secs(10)))
}
