use std::sync::Arc;

use actix_web::Error;
use actix_web::HttpResponse;
use actix_web::get;
use actix_web::web;
use async_trait::async_trait;
use serde::Deserialize;

use crate::balancer::agent_controller::AgentController;
use crate::balancer::agent_controller_pool::AgentControllerPool;
use crate::balancer::controls_manages_senders_endpoint::ControlsManagesSendersEndpoint;
use crate::balancer::management_service::app_data::AppData;
use crate::balancer::manages_senders_controller::ManagesSendersController;
use crate::balancer::model_metadata_sender_collection::ModelMetadataSenderCollection;

pub fn register(cfg: &mut web::ServiceConfig) {
    cfg.service(respond);
}

struct GetModelMetadataController {
    agent_controller_pool: Arc<AgentControllerPool>,
    agent_id: String,
}

#[async_trait]
impl ControlsManagesSendersEndpoint for GetModelMetadataController {
    type SenderCollection = ModelMetadataSenderCollection;

    fn get_agent_controller_pool(&self) -> Arc<AgentControllerPool> {
        self.agent_controller_pool.clone()
    }

    fn get_agent_id(&self) -> String {
        self.agent_id.clone()
    }

    async fn get_manages_senders_controller(
        &self,
        agent_controller: Arc<AgentController>,
    ) -> anyhow::Result<ManagesSendersController<Self::SenderCollection>> {
        agent_controller.get_model_metadata().await
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PathParams {
    agent_id: String,
}

#[get("/api/v1/agent/{agent_id}/model_metadata")]
async fn respond(
    app_data: web::Data<AppData>,
    params: web::Path<PathParams>,
) -> Result<HttpResponse, Error> {
    let controller = GetModelMetadataController {
        agent_controller_pool: app_data.agent_controller_pool.clone(),
        agent_id: params.agent_id.clone(),
    };

    controller.respond().await
}
