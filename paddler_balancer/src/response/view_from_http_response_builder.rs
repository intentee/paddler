use actix_web::HttpResponse;
use actix_web::HttpResponseBuilder;
use actix_web::Result;
use actix_web::error::ErrorInternalServerError;
use askama::Template;

pub fn view_from_http_response_builder<TTemplate: Template>(
    mut http_response_builder: HttpResponseBuilder,
    template: TTemplate,
) -> Result<HttpResponse> {
    let rendered = template.render().map_err(ErrorInternalServerError)?;

    Ok(http_response_builder
        .content_type("text/html; charset=utf-8")
        .body(rendered))
}

#[cfg(test)]
mod tests {
    use actix_web::HttpResponse;
    use actix_web::http::StatusCode;
    use actix_web::http::header::CONTENT_TYPE;
    use askama::Template;

    use super::view_from_http_response_builder;

    #[derive(Template)]
    #[template(ext = "html", source = "<p>rendered</p>")]
    struct RenderingTemplate;

    #[derive(Template)]
    #[template(
        ext = "html",
        source = "{{ (\"absent_runtime_value\"|value::<String>)? }}"
    )]
    struct MissingRuntimeValueTemplate;

    #[test]
    fn renders_template_into_ok_html_response() {
        let http_response =
            view_from_http_response_builder(HttpResponse::Ok(), RenderingTemplate).unwrap();

        assert_eq!(http_response.status(), StatusCode::OK);
        assert_eq!(
            http_response
                .headers()
                .get(CONTENT_TYPE)
                .unwrap()
                .to_str()
                .unwrap(),
            "text/html; charset=utf-8",
        );
    }

    #[test]
    fn maps_render_failure_to_internal_server_error() {
        let render_error =
            view_from_http_response_builder(HttpResponse::Ok(), MissingRuntimeValueTemplate)
                .unwrap_err();

        assert_eq!(
            render_error.as_response_error().status_code(),
            StatusCode::INTERNAL_SERVER_ERROR,
        );
    }
}
