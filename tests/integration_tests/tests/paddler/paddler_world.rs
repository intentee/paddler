use cucumber::World;
use dashmap::DashMap;
use reqwest::Response;

use crate::agents_collection::AgentsCollection;
use crate::balancer_instance::BalancerInstance;
use crate::llamacpp_instance_collection::LlamaCppInstanceCollection;
use crate::request_builder::RequestBuilder;
use crate::statsd_instance::StatsdInstance;

#[derive(Debug, Default, World)]
pub struct PaddlerWorld {
    pub agents: AgentsCollection,
    pub balancer: BalancerInstance,
    pub llamas: LlamaCppInstanceCollection,
    pub request_builder: RequestBuilder,
    pub responses: DashMap<String, Response>,
    pub statsd: StatsdInstance,
}

impl PaddlerWorld {
    pub async fn cleanup(&mut self) {
        self.agents.cleanup().await;
        self.balancer.cleanup().await;
        self.llamas.cleanup().await;
        self.request_builder.cleanup();
        self.responses.clear();
        self.statsd.cleanup().await;
    }
}
