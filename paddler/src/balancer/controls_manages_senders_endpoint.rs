use std::sync::Arc;

use actix_web::Error;
use actix_web::HttpResponse;
use async_trait::async_trait;
use tokio::time::Duration;
use tokio::time::sleep;

use crate::balancer::agent_controller::AgentController;
use crate::balancer::agent_controller_pool::AgentControllerPool;
use crate::balancer::manages_senders::ManagesSenders;
use crate::balancer::manages_senders_controller::ManagesSendersController;

const TIMEOUT: Duration = Duration::from_secs(3);

#[async_trait]
pub trait ControlsManagesSendersEndpoint {
    type SenderCollection: ManagesSenders + Send + Sync + 'static;

    fn get_agent_controller_pool(&self) -> Arc<AgentControllerPool>;

    fn get_agent_id(&self) -> String;

    async fn get_manages_senders_controller(
        &self,
        agent_controller: Arc<AgentController>,
    ) -> anyhow::Result<ManagesSendersController<Self::SenderCollection>>;

    async fn respond(&self) -> Result<HttpResponse, Error> {
        let agent_controller_pool = self.get_agent_controller_pool();
        let agent_id = self.get_agent_id();
        let agent_controller = match agent_controller_pool.get_agent_controller(&agent_id) {
            Some(agent_controller) => agent_controller,
            None => {
                return Ok(HttpResponse::NotFound().finish());
            }
        };

        let mut connection_close_rx = agent_controller.connection_close_rx.resubscribe();

        match self.get_manages_senders_controller(agent_controller).await {
            Ok(mut receive_response_controller) => {
                tokio::select! {
                    _ = connection_close_rx.recv() => Ok(HttpResponse::BadGateway().finish()),
                    _ = sleep(TIMEOUT) => Ok(HttpResponse::GatewayTimeout().finish()),
                    response = receive_response_controller.response_rx.recv() => match response {
                        Some(existing_response) => Ok(HttpResponse::Ok().json(existing_response)),
                        None => Ok(HttpResponse::NotFound().finish()),
                    },
                }
            }
            Err(err) => Ok(HttpResponse::InternalServerError().body(format!("{err}"))),
        }
    }
}
