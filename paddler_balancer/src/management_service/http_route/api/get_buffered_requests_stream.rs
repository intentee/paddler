use actix_web::Responder;
use actix_web::get;
use actix_web::web;

use crate::management_service::app_data::AppData;
use crate::sse_response_from_snapshots::sse_response_from_snapshots;

pub fn register(cfg: &mut web::ServiceConfig) {
    cfg.service(respond);
}

#[get("/api/v1/buffered_requests/stream")]
async fn respond(app_data: web::Data<AppData>) -> impl Responder {
    sse_response_from_snapshots(
        app_data.buffered_request_manager.clone(),
        app_data.shutdown.clone(),
    )
}
