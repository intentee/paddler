use paddler_model_source::desired_model_resolver::DesiredModelResolver;

pub struct AgentBootstrapConfig {
    pub agent_name: Option<String>,
    pub desired_model_resolver: DesiredModelResolver,
    pub management_address: String,
    pub slots: u16,
}
