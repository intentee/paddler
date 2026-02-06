use actix_web::HttpResponse;
use actix_web::Responder;
use actix_web::get;
use actix_web::web;

const FAVICON: &[u8] = include_bytes!("../../../../../resources/images/favicon.svg");

pub fn register(cfg: &mut web::ServiceConfig) {
    cfg.service(respond);
}

#[get("/favicon.ico")]
async fn respond() -> impl Responder {
    HttpResponse::Ok()
        .content_type("image/svg+xml")
        .body(FAVICON)
}
