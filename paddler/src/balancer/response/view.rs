use actix_web::HttpResponse;
use actix_web::Result;
use askama::Template;

use super::view_from_http_response_builder::view_from_http_response_builder;

pub fn view<TTemplate: Template>(template: TTemplate) -> Result<HttpResponse> {
    view_from_http_response_builder(HttpResponse::Ok(), template)
}
