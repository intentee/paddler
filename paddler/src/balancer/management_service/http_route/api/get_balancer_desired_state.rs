use actix_web::Error;
use actix_web::HttpResponse;
use actix_web::Responder;
use actix_web::error::ErrorInternalServerError;
use actix_web::get;
use actix_web::web;

use crate::balancer::management_service::app_data::AppData;

pub fn register(cfg: &mut web::ServiceConfig) {
    cfg.service(respond);
}

#[get("/api/v1/balancer_desired_state")]
async fn respond(app_data: web::Data<AppData>) -> Result<impl Responder, Error> {
    let desired_state = app_data
        .state_database
        .read_balancer_desired_state()
        .await
        .map_err(ErrorInternalServerError)?;

    Ok(HttpResponse::Ok().json(desired_state))
}
