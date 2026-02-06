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
