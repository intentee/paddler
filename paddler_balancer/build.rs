#[cfg(feature = "web_admin_panel")]
use std::env::var_os;
#[cfg(feature = "web_admin_panel")]
use std::io::stdout;
#[cfg(feature = "web_admin_panel")]
use std::path::Path;

#[cfg(feature = "web_admin_panel")]
use paddler_dashboard_assets::dashboard_assets_build::DashboardAssetsBuild;
#[cfg(feature = "web_admin_panel")]
use paddler_dashboard_assets::dashboard_assets_error::DashboardAssetsError;

#[cfg(feature = "web_admin_panel")]
fn main() -> Result<(), DashboardAssetsError> {
    DashboardAssetsBuild {
        metafile_path: Path::new(env!("CARGO_MANIFEST_DIR")).join("../esbuild-meta.json"),
        out_dir: var_os("OUT_DIR"),
    }
    .run(stdout())
}

#[cfg(not(feature = "web_admin_panel"))]
fn main() {}
