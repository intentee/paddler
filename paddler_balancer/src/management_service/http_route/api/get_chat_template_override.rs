use actix_web::HttpResponse;
use actix_web::web;
use actix_web::web::get;

use paddler_messaging::api_path::ApiPath;

use crate::agent_id_path_params::AgentIdPathParams;
use crate::constants::AGENT_ID_ROUTE_SEGMENT;
use crate::management_service::app_data::AppData;

async fn respond(
    app_data: web::Data<AppData>,
    params: web::Path<AgentIdPathParams>,
) -> HttpResponse {
    app_data
        .respond_with_agent_response(&params.agent_id, |agent_controller| {
            agent_controller.get_chat_template_override()
        })
        .await
}

pub fn get_chat_template_override(cfg: &mut web::ServiceConfig) {
    cfg.route(
        &ApiPath::agent_chat_template_override(AGENT_ID_ROUTE_SEGMENT),
        get().to(respond),
    );
}
