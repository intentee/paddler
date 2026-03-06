use std::collections::BTreeSet;
use std::convert::Infallible;
use std::time::Duration;

use actix_web::Error;
use actix_web::Responder;
use actix_web::get;
use actix_web::web;
use actix_web_lab::sse;
use log::error;
use paddler_types::agent_issue::AgentIssue;
use paddler_types::agent_issue_severity::AgentIssueSeverity;
use paddler_types::agent_issue_type::AgentIssueType;
use paddler_types::agent_state_application_status::AgentStateApplicationStatus;

use crate::balancer::management_service::app_data::AppData;
use crate::produces_snapshot::ProducesSnapshot as _;

pub fn register(cfg: &mut web::ServiceConfig) {
    cfg.service(respond);
}

#[get("/api/v1/agents/stream")]
async fn respond(app_data: web::Data<AppData>) -> Result<impl Responder, Error> {
    let event_stream = async_stream::stream! {
        let send_event = |info| {
            match serde_json::to_string(&info) {
                Ok(json) => Some(Ok::<_, Infallible>(sse::Event::Data(sse::Data::new(json)))),
                Err(err) => {
                    error!("Failed to serialize pool info: {err}");
                    None
                }
            }
        };

        loop {
            match app_data.agent_controller_pool.make_snapshot() {
                Ok(mut agent_controller_pool_snapshot) => {
                        agent_controller_pool_snapshot.agents.iter_mut().for_each(|agent_controller_snapshot| {
        let mut issues = BTreeSet::new();

        if agent_controller_snapshot.state_application_status == AgentStateApplicationStatus::Applied {
            issues.insert(AgentIssue {
                type_: AgentIssueType::UnableToFindChatTemplate("Unable to find chat template".to_string()),
                severity: AgentIssueSeverity::Warning,
            });
        }

        agent_controller_snapshot.issues = issues.into_iter().collect();
    });

                    if let Some(event) = send_event(agent_controller_pool_snapshot) {
                        yield event;
                    }
                }
                Err(err) => error!("Failed to get agent controller pool snapshot: {err}"),
            }

            app_data.agent_controller_pool.update_notifier.notified().await;
        }
    };

    Ok(sse::Sse::from_stream(event_stream).with_keep_alive(Duration::from_secs(10)))
}
