use actix_web::Responder;
use actix_web::web;
use actix_web::web::get;

use paddler_messaging::api_path::ApiPath;

use crate::management_service::app_data::AppData;
use crate::sse_response_from_snapshots::sse_response_from_snapshots;

async fn respond(app_data: web::Data<AppData>) -> impl Responder {
    sse_response_from_snapshots(
        app_data.agent_controller_pool.clone(),
        app_data.shutdown.clone(),
    )
}

pub fn get_agents_stream(cfg: &mut web::ServiceConfig) {
    cfg.route(ApiPath::AGENTS_STREAM, get().to(respond));
}
