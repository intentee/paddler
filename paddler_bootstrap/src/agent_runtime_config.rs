use paddler_model_source::desired_model_resolver::DesiredModelResolver;

pub struct AgentRuntimeConfig {
    pub agent_name: Option<String>,
    pub desired_model_resolver: DesiredModelResolver,
    pub slots: u16,
}
