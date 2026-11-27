use actix_web::Responder;
use actix_web::web::ServiceConfig;
use actix_web::web::get;

use paddler_messaging::api_path::ApiPath;

async fn respond() -> impl Responder {
    "OK"
}

pub fn get_health(cfg: &mut ServiceConfig) {
    cfg.route(ApiPath::health(), get().to(respond));
}
