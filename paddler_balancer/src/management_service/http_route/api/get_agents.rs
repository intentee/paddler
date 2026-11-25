use actix_web::HttpResponse;
use actix_web::web;
use actix_web::web::get;

use paddler_messaging::constants::API_PATH_AGENTS;
use paddler_messaging::produces_snapshot::ProducesSnapshot as _;

use crate::management_service::app_data::AppData;

async fn respond(app_data: web::Data<AppData>) -> HttpResponse {
    HttpResponse::Ok().json(app_data.agent_controller_pool.make_snapshot())
}

pub fn get_agents(cfg: &mut web::ServiceConfig) {
    cfg.route(API_PATH_AGENTS, get().to(respond));
}
