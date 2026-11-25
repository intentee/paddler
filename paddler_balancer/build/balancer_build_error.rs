use thiserror::Error;

use paddler_balancer_dashboard_assets::dashboard_assets_error::DashboardAssetsError;

#[derive(Debug, Error)]
pub enum BalancerBuildError {
    #[error("unable to build the dashboard assets")]
    DashboardAssetsUnbuildable(#[from] DashboardAssetsError),
}
