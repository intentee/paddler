use actix_web::Error;
use actix_web::HttpResponse;
use actix_web::error::ErrorInternalServerError;
use actix_web::get;
use actix_web::web;

use crate::balancer::management_service::app_data::AppData;
use crate::produces_snapshot::ProducesSnapshot as _;

pub fn register(cfg: &mut web::ServiceConfig) {
    cfg.service(respond);
}

#[get("/api/v1/buffered_requests")]
async fn respond(app_data: web::Data<AppData>) -> Result<HttpResponse, Error> {
    Ok(HttpResponse::Ok().json(
        app_data
            .buffered_request_manager
            .make_snapshot()
            .map_err(ErrorInternalServerError)?,
    ))
}
