use std::env::var_os;
use std::io::stdout;
use std::path::Path;

use paddler_dashboard_assets::dashboard_assets_build::DashboardAssetsBuild;
use paddler_dashboard_assets::dashboard_assets_error::DashboardAssetsError;
use paddler_dashboard_assets::web_admin_panel_feature::WebAdminPanelFeature;

fn main() -> Result<(), DashboardAssetsError> {
    DashboardAssetsBuild {
        metafile_path: Path::new(env!("CARGO_MANIFEST_DIR")).join("../esbuild-meta.json"),
        out_dir: var_os("OUT_DIR"),
        web_admin_panel: WebAdminPanelFeature::from_cargo_feature_variable(
            var_os("CARGO_FEATURE_WEB_ADMIN_PANEL").as_ref(),
        ),
    }
    .run(stdout())
}
