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

#[cfg(test)]
mod tests {
    use actix_web::App;
    use actix_web::http::StatusCode;
    use actix_web::http::header::CONTENT_TYPE;
    use actix_web::test;
    use mime_guess::from_path;

    use super::register;
    use crate::static_files::StaticFiles;

    fn any_embedded_file_name() -> String {
        StaticFiles::iter()
            .next()
            .map(|file_name| file_name.as_ref().to_owned())
            .unwrap()
    }

    #[actix_web::test]
    async fn serves_embedded_file_with_guessed_content_type() {
        let existing_file_path = any_embedded_file_name();

        let app = test::init_service(App::new().configure(register)).await;
        let request = test::TestRequest::get()
            .uri(&format!("/static/{existing_file_path}"))
            .to_request();
        let response = test::call_service(&app, request).await;

        assert_eq!(response.status(), StatusCode::OK);

        let content_type = response.headers().get(CONTENT_TYPE).unwrap();
        let expected_content_type = from_path(&existing_file_path).first_or_octet_stream();

        assert_eq!(content_type, expected_content_type.as_ref());

        let expected_body = StaticFiles::get(&existing_file_path)
            .unwrap()
            .data
            .into_owned();
        let body = test::read_body(response).await;

        assert_eq!(body.as_ref(), expected_body.as_slice());
    }

    #[actix_web::test]
    async fn responds_with_not_found_for_missing_file() {
        let app = test::init_service(App::new().configure(register)).await;
        let request = test::TestRequest::get()
            .uri("/static/this_file_does_not_exist.txt")
            .to_request();
        let response = test::call_service(&app, request).await;

        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        let body = test::read_body(response).await;

        assert_eq!(body.as_ref(), b"File not found");
    }
}
