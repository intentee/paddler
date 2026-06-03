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

#[cfg(test)]
mod tests {
    use actix_web::App;
    use actix_web::http::StatusCode;
    use actix_web::http::header;
    use actix_web::test;

    use super::FAVICON;
    use super::register;

    #[actix_web::test]
    async fn serves_embedded_favicon_as_svg() {
        let app = test::init_service(App::new().configure(register)).await;
        let request = test::TestRequest::get().uri("/favicon.ico").to_request();
        let response = test::call_service(&app, request).await;

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

        let body = test::read_body(response).await;

        assert_eq!(body.as_ref(), FAVICON);
    }
}
