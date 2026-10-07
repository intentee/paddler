use actix_web::HttpResponse;
use actix_web::http::StatusCode;
use actix_web::web;
use actix_web::web::post;

use paddler_typesafe_translation::system_one_request::SystemOneRequest;

use crate::compatibility::compatibility_app_data::CompatibilityAppData;
use crate::compatibility::typesafe_service::system_one_http_response::system_one_http_response;
use crate::compatibility::typesafe_service::typesafe_api_path::TypeSafeApiPath;
use crate::compatibility::typesafe_service::typesafe_error_body::TypeSafeErrorBody;
use crate::compatibility::typesafe_service::typesafe_json_config::typesafe_json_config;

async fn respond(
    app_data: web::Data<CompatibilityAppData>,
    system_one_request: web::Json<SystemOneRequest>,
) -> HttpResponse {
    match system_one_request.into_inner().translate() {
        Ok(translated) => {
            let decision_results = app_data.agent_result_stream(translated.decide_params.clone());

            system_one_http_response(&translated, decision_results).await
        }
        Err(translation_error) => TypeSafeErrorBody {
            detail: translation_error.to_string(),
        }
        .into_http_response(StatusCode::UNPROCESSABLE_ENTITY),
    }
}

pub fn post_system_one(cfg: &mut web::ServiceConfig) {
    cfg.app_data(typesafe_json_config())
        .route(TypeSafeApiPath::SYSTEM_ONE, post().to(respond));
}
