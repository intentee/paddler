use actix_web::HttpResponse;
use actix_web::Responder;
use actix_web::get;
use actix_web::web;
use mime_guess::from_path;

use crate::static_files::StaticFiles;

pub fn register(cfg: &mut web::ServiceConfig) {
    cfg.service(respond);
}

#[get("/static/{path:.*}")]
async fn respond(path: web::Path<String>) -> impl Responder {
    let path = path.into_inner();

    match StaticFiles::get(path.as_str()) {
        Some(content) => HttpResponse::Ok()
            .content_type(from_path(path).first_or_octet_stream().as_ref())
            .body(content.data.into_owned()),
        None => HttpResponse::NotFound().body("File not found"),
    }
}
