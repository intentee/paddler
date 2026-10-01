use actix_web::HttpResponse;
use actix_web::web;
use actix_web::web::get;

use paddler_messaging::api_path::ApiPath;

use crate::agent_id_path_params::AgentIdPathParams;
use crate::management_service::app_data::AppData;
use crate::respond_with_agent_response::respond_with_agent_response;

async fn respond(
    app_data: web::Data<AppData>,
    params: web::Path<AgentIdPathParams>,
) -> HttpResponse {
    respond_with_agent_response(
        &app_data.agent_controller_pool,
        &params.agent_id,
        |agent_controller| agent_controller.get_chat_template_override(),
    )
    .await
}

pub fn get_chat_template_override(cfg: &mut web::ServiceConfig) {
    cfg.route(
        &ApiPath::agent_chat_template_override(AgentIdPathParams::ROUTE_SEGMENT),
        get().to(respond),
    );
}
