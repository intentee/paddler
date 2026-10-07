use actix_web::HttpResponse;
use actix_web::http::StatusCode;
use serde::Serialize;

#[derive(Serialize)]
pub struct TypeSafeErrorBody {
    pub detail: String,
}

impl TypeSafeErrorBody {
    #[must_use]
    pub fn into_http_response(self, status_code: StatusCode) -> HttpResponse {
        HttpResponse::build(status_code).json(self)
    }
}
