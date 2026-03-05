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
use paddler_types::agent_state_application_status::AgentStateApplicationStatus;
use paddler_types::issue_severity::IssueSeverity;
use paddler_types::issue_type::IssueType;

use crate::balancer::management_service::app_data::AppData;

pub fn register(cfg: &mut web::ServiceConfig) {
    cfg.service(respond);
}

#[get("/api/v1/agents")]
async fn respond(app_data: web::Data<AppData>) -> Result<HttpResponse, Error> {
    // let mut snapshot = app_data
    //     .agent_controller_pool
    //     .make_snapshot()
    //     .map_err(ErrorInternalServerError)?;

    let snapshot = AgentControllerPoolSnapshot {
        agents: mock_agents(),
    };

    Ok(HttpResponse::Ok().json(snapshot))
}

pub fn mock_agents() -> Vec<AgentControllerSnapshot> {
    vec![
        AgentControllerSnapshot {
            desired_slots_total: 2,
            download_current: 0,
            download_filename: None,
            download_total: 0,
            id: "mock-agent-error".to_string(),
            issues: BTreeSet::from([AgentIssue {
                type_: IssueType::ModelCannotBeLoaded("/models/broken-model.gguf".to_string()),
                severity: IssueSeverity::Error,
            }]),
            model_path: Some("/models/broken-model.gguf".to_string()),
            name: Some("agent-with-error-agent-with-erroragent-with-error-agent-with-error-agent-with-error-agent-with-error".to_string()),
            slots_processing: 1,
            slots_total: 2,
            state_application_status: AgentStateApplicationStatus::AttemptedAndNotAppliable,
            uses_chat_template_override: true,
        },
        AgentControllerSnapshot {
            desired_slots_total: 2,
            download_current: 0,
            download_filename: None,
            download_total: 0,
            id: "mock-agent-warning".to_string(),
            issues: BTreeSet::from([AgentIssue {
                type_: IssueType::UnableToFindChatTemplate("/models/embed-model.gguf".to_string()),
                severity: IssueSeverity::Warning,
            }]),
            model_path: Some("/models/embed-model.gguf".to_string()),
            name: Some("agent-with-warning-agent-with-warningagent-with-warning-agent-with-warning-agent-with-warning-agent-with-warning".to_string()),
            slots_processing: 1,
            slots_total: 2,
            state_application_status: AgentStateApplicationStatus::Applied,
            uses_chat_template_override: false,
        },
        AgentControllerSnapshot {
            desired_slots_total: 2,
            download_current: 0,
            download_filename: None,
            download_total: 0,
            id: "mock-agent-mixed".to_string(),
            issues: BTreeSet::from([
                AgentIssue {
                    type_: IssueType::ChatTemplateDoesNotCompile(
                        ChatTemplateDoesNotCompileParams {
                            error: "syntax error in template".to_string(),
                            template_content: "{{ broken template".to_string(),
                        },
                    ),
                    severity: IssueSeverity::Warning,
                },
                AgentIssue {
                    type_: IssueType::SlotCannotStart(
                        paddler_types::agent_issue_params::SlotCannotStartParams {
                            error: "out of memory".to_string(),
                            slot_index: 1,
                        },
                    ),
                    severity: IssueSeverity::Error,
                },
                AgentIssue {
                    type_: IssueType::SlotCannotStart(
                        paddler_types::agent_issue_params::SlotCannotStartParams {
                            error: "out of disk".to_string(),
                            slot_index: 1,
                        },
                    ),
                    severity: IssueSeverity::Error,
                },
            ]),
            model_path: Some("/models/large-model.gguf".to_string()),
            name: Some("agent-with-mixed-issues-agent-with-mixed-issuesagent-with-mixed-issues-agent-with-mixed-issues-agent-with-mixed-issues-agent-with-mixed-issues".to_string()),
            slots_processing: 1,
            slots_total: 2,
            state_application_status: AgentStateApplicationStatus::AttemptedAndNotAppliable,
            uses_chat_template_override: true,
        },
    ]
}
