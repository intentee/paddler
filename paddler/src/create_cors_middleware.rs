use std::sync::Arc;

use actix_cors::Cors;
use actix_web::http::header;

pub fn create_cors_middleware(allowed_hosts: Arc<Vec<String>>) -> Cors {
    let mut cors = Cors::default()
        .allowed_methods(vec!["DELETE", "GET", "POST", "PUT", "OPTIONS"])
        .allowed_headers(vec![
            header::ACCEPT,
            header::AUTHORIZATION,
            header::CONTENT_TYPE,
        ])
        .max_age(3600);

    for host in allowed_hosts.iter() {
        cors = cors.allowed_origin(host);
    }

    cors
}
