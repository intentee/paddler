use actix_web::HttpResponse;
use actix_web::Responder;
use actix_web::web;
use actix_web::web::get;

const FAVICON: &[u8] = include_bytes!("../../../../resources/images/favicon.svg");

async fn respond() -> impl Responder {
    HttpResponse::Ok()
        .content_type("image/svg+xml")
        .body(FAVICON)
}

pub fn favicon(cfg: &mut web::ServiceConfig) {
    cfg.route("/favicon.ico", get().to(respond));
}

#[cfg(test)]
mod tests {
    use actix_web::App;
    use actix_web::http::StatusCode;
    use actix_web::http::header;
    use actix_web::test::TestRequest;
    use actix_web::test::call_service;
    use actix_web::test::init_service;
    use actix_web::test::read_body;

    use super::FAVICON;
    use super::favicon;

    #[actix_web::test]
    async fn serves_embedded_favicon_as_svg() {
        let app = init_service(App::new().configure(favicon)).await;
        let request = TestRequest::get().uri("/favicon.ico").to_request();
        let response = call_service(&app, request).await;

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response
                .headers()
                .get(header::CONTENT_TYPE)
                .unwrap()
                .to_str()
                .unwrap(),
            "image/svg+xml"
        );

        let body = read_body(response).await;

        assert_eq!(body.as_ref(), FAVICON);
    }
}
