use actix_web::Error;
use actix_web::HttpResponse;
use actix_web::Responder;
use actix_web::error::ErrorInternalServerError;
use actix_web::put;
use actix_web::web;

use crate::balancer::management_service::app_data::AppData;
use crate::balancer_desired_state::BalancerDesiredState;

pub fn register(cfg: &mut web::ServiceConfig) {
    cfg.service(respond);
}

#[put("/api/v1/balancer_desired_state")]
async fn respond(
    app_data: web::Data<AppData>,
    balancer_desired_state: web::Json<BalancerDesiredState>,
) -> Result<impl Responder, Error> {
    let balancer_desired_state_inner = balancer_desired_state.into_inner();

    app_data
        .state_database
        .store_balancer_desired_state(&balancer_desired_state_inner)
        .await
        .map_err(ErrorInternalServerError)?;

    Ok(HttpResponse::NoContent().finish())
}
