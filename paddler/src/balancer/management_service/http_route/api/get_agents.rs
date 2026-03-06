use std::collections::BTreeSet;

use actix_web::Error;
use actix_web::HttpResponse;
use actix_web::error::ErrorInternalServerError;
use actix_web::get;
use actix_web::web;
use paddler_types::agent_controller_pool_snapshot::AgentControllerPoolSnapshot;
use paddler_types::agent_controller_snapshot::AgentControllerSnapshot;
use paddler_types::agent_issue::AgentIssue;
use paddler_types::agent_issue_params::ChatTemplateDoesNotCompileParams;
use paddler_types::agent_issue_severity::AgentIssueSeverity;
use paddler_types::agent_issue_type::AgentIssueType;
use paddler_types::agent_state_application_status::AgentStateApplicationStatus;

use crate::balancer::management_service::app_data::AppData;
use crate::produces_snapshot::ProducesSnapshot;

pub fn register(cfg: &mut web::ServiceConfig) {
    cfg.service(respond);
}

#[get("/api/v1/agents")]
async fn respond(app_data: web::Data<AppData>) -> Result<HttpResponse, Error> {
    let mut snapshot = app_data
        .agent_controller_pool
        .make_snapshot()
        .map_err(ErrorInternalServerError)?;

    snapshot
        .agents
        .iter_mut()
        .for_each(|agent_controller_snapshot| {
            let mut issues = BTreeSet::new();

            if agent_controller_snapshot.state_application_status
                == AgentStateApplicationStatus::Applied
            {
                issues.insert(AgentIssue {
                    type_: AgentIssueType::UnableToFindChatTemplate(
                        "Unable to find chat template".to_string(),
                    ),
                    severity: AgentIssueSeverity::Warning,
                });
            }

            agent_controller_snapshot.issues = issues.into_iter().collect();
        });

    Ok(HttpResponse::Ok().json(snapshot))
}
