use std::error::Error;

use actix_web::Responder;
use actix_web::get;
use actix_web::web::ServiceConfig;

pub fn register(cfg: &mut ServiceConfig) {
    cfg.service(respond);
}

#[get("/health")]
async fn respond() -> Result<impl Responder, Box<dyn Error>> {
    Ok("OK")
}
