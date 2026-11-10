use std::error::Error;

use actix_web::Responder;
use actix_web::web::ServiceConfig;
use actix_web::web::get;

use paddler_messaging::constants::API_PATH_HEALTH;

async fn respond() -> Result<impl Responder, Box<dyn Error>> {
    Ok("OK")
}

pub fn get_health(cfg: &mut ServiceConfig) {
    cfg.route(API_PATH_HEALTH, get().to(respond));
}
