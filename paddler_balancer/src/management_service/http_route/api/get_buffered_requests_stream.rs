use actix_web::Responder;
use actix_web::web;
use actix_web::web::get;

use paddler_messaging::api_path::ApiPath;

use crate::management_service::app_data::AppData;
use crate::sse_response_from_snapshots::sse_response_from_snapshots;

async fn respond(app_data: web::Data<AppData>) -> impl Responder {
    sse_response_from_snapshots(
        app_data.buffered_request_manager.clone(),
        app_data.shutdown.clone(),
    )
}

pub fn get_buffered_requests_stream(cfg: &mut web::ServiceConfig) {
    cfg.route(ApiPath::BUFFERED_REQUESTS_STREAM, get().to(respond));
}
