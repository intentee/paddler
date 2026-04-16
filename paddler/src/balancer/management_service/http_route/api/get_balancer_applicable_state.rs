use actix_web::Error;
use actix_web::HttpResponse;
use actix_web::Responder;
use actix_web::get;
use actix_web::web;

use crate::balancer::management_service::app_data::AppData;

pub fn register(cfg: &mut web::ServiceConfig) {
    cfg.service(respond);
}

#[get("/api/v1/balancer_applicable_state")]
async fn respond(app_data: web::Data<AppData>) -> Result<impl Responder, Error> {
    let applicable_state = app_data
        .balancer_applicable_state_holder
        .get_agent_desired_state();

    Ok(HttpResponse::Ok().json(applicable_state))
}
