use actix_web::HttpResponse;
use actix_web::Result;
use askama::Template;

use super::view_from_http_response_builder::view_from_http_response_builder;

pub fn view<TTemplate: Template>(template: TTemplate) -> Result<HttpResponse> {
    view_from_http_response_builder(HttpResponse::Ok(), template)
}

#[cfg(test)]
mod tests {
    use actix_web::http::StatusCode;
    use actix_web::http::header::CONTENT_TYPE;
    use askama::Template;

    use super::view;

    #[derive(Template)]
    #[template(ext = "html", source = "<p>{{ greeting }}</p>")]
    struct GreetingTemplate {
        greeting: String,
    }

    #[test]
    fn responds_with_ok_status() {
        let response = view(GreetingTemplate {
            greeting: "hello".to_owned(),
        })
        .unwrap();

        assert_eq!(StatusCode::OK, response.status());
    }

    #[test]
    fn responds_with_html_content_type() {
        let response = view(GreetingTemplate {
            greeting: "hello".to_owned(),
        })
        .unwrap();

        let content_type = response
            .headers()
            .get(CONTENT_TYPE)
            .unwrap()
            .to_str()
            .unwrap();

        assert_eq!("text/html; charset=utf-8", content_type);
    }
}
