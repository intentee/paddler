#[cfg(feature = "web_admin_panel")]
use paddler_balancer_dashboard_assets::dashboard_assets_html_file_name;

pub const AGENT_ID_ROUTE_SEGMENT: &str = "{agent_id}";
#[cfg(feature = "web_admin_panel")]
pub const DASHBOARD_ASSETS_HTML: &str = include_str!(concat!(
    env!("OUT_DIR"),
    "/",
    dashboard_assets_html_file_name!()
));
pub const MAX_WEBSOCKET_MESSAGE_SIZE: usize = 50 * 1024 * 1024;
