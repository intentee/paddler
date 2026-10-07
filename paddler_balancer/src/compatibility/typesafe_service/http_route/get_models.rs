use actix_web::HttpResponse;
use actix_web::web;
use actix_web::web::get;

use crate::compatibility::compatibility_app_data::CompatibilityAppData;
use crate::compatibility::typesafe_service::typesafe_api_path::TypeSafeApiPath;
use crate::compatibility::typesafe_service::typesafe_models::TypeSafeModels;

async fn respond(app_data: web::Data<CompatibilityAppData>) -> HttpResponse {
    HttpResponse::Ok().json(TypeSafeModels::from(
        app_data
            .balancer_applicable_state_holder
            .get_agent_desired_state()
            .model,
    ))
}

pub fn get_models(cfg: &mut web::ServiceConfig) {
    cfg.route(TypeSafeApiPath::MODELS, get().to(respond));
}
