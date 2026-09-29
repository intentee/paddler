#[derive(Default)]
pub struct JoinBalancerFormData {
    pub agent_name: String,
    pub balancer_address: String,
    pub balancer_address_error: Option<String>,
    pub slots_count: String,
    pub slots_error: Option<String>,
}

impl JoinBalancerFormData {
    #[must_use]
    pub fn entered_agent_name(&self) -> Option<String> {
        (!self.agent_name.is_empty()).then(|| self.agent_name.clone())
    }
}
