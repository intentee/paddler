use paddler_cache_dir::cache_dir::CacheDir;

#[derive(Debug, Eq, PartialEq)]
pub struct AgentBootstrapConfig {
    pub agent_name: Option<String>,
    pub cache_dir: CacheDir,
    pub management_address: String,
    pub slots: u16,
}
