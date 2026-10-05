use std::time::Duration;

pub struct BalancerDefaults;

impl BalancerDefaults {
    pub const BUFFERED_REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
    pub const INFERENCE_ITEM_TIMEOUT: Duration = Duration::from_secs(30);
    pub const INFERENCE_PORT: u16 = 8061;
    pub const MANAGEMENT_PORT: u16 = 8060;
    pub const MAX_BUFFERED_REQUESTS: u64 = 30;
    pub const STATSD_PREFIX: &str = "paddler_";
    pub const STATSD_REPORTING_INTERVAL: Duration = Duration::from_secs(10);
    pub const WEB_ADMIN_PANEL_PORT: u16 = 8062;
}
