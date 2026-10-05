use crate::running_balancer_snapshot::RunningBalancerSnapshot;

pub struct RunningBalancerData {
    pub balancer_address: String,
    pub snapshot: RunningBalancerSnapshot,
    pub stopping: bool,
    pub web_admin_panel_address: Option<String>,
}
