use std::ffi::OsString;
use std::fs::read_to_string;
use std::fs::write;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::str::FromStr as _;

use esbuild_metafile::esbuild_metafile::EsbuildMetafile;

use crate::dashboard_assets::DashboardAssets;
use crate::dashboard_assets_error::DashboardAssetsError;

const DASHBOARD_ASSETS_HTML_FILE_NAME: &str = "dashboard_assets.html";

pub struct DashboardAssetsBuild {
    pub metafile_path: PathBuf,
    pub out_dir: Option<OsString>,
}

impl DashboardAssetsBuild {
    pub fn run(&self, mut cargo_instructions: impl Write) -> Result<(), DashboardAssetsError> {
        let metafile_contents = read_to_string(&self.metafile_path).map_err(|source| {
            DashboardAssetsError::MetafileUnreadable {
                path: self.metafile_path.clone(),
                source,
            }
        })?;
        let esbuild_metafile = EsbuildMetafile::from_str(&metafile_contents).map_err(|source| {
            DashboardAssetsError::MetafileMalformed {
                path: self.metafile_path.clone(),
                source,
            }
        })?;
        let DashboardAssets { html } = DashboardAssets::of(&esbuild_metafile)?;
        let html_path = Path::new(
            self.out_dir
                .as_ref()
                .ok_or(DashboardAssetsError::OutDirMissing)?,
        )
        .join(DASHBOARD_ASSETS_HTML_FILE_NAME);

        write(&html_path, html).map_err(|source| DashboardAssetsError::HtmlUnwritable {
            path: html_path.clone(),
            source,
        })?;

        write!(
            cargo_instructions,
            "cargo::rerun-if-changed={}\ncargo::rustc-env=PADDLER_DASHBOARD_ASSETS_HTML={}\n",
            self.metafile_path.display(),
            html_path.display()
        )
        .map_err(DashboardAssetsError::CargoInstructionsUnwritable)
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    #[cfg(target_os = "linux")]
    use std::fs::OpenOptions;
    use std::fs::read_to_string;
    use std::fs::write;
    use std::io::ErrorKind;
    use std::mem::discriminant;
    use std::path::Path;
    use std::path::PathBuf;
    use std::str::FromStr as _;

    use esbuild_metafile::esbuild_metafile::EsbuildMetafile;
    use tempfile::TempDir;

    use super::DashboardAssetsBuild;
    use crate::dashboard_assets::DashboardAssets;
    use crate::dashboard_assets_error::DashboardAssetsError;

    const DASHBOARD_METAFILE: &str = include_str!("../fixtures/dashboard_metafile.json");

    fn dashboard_metafile_path() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/dashboard_metafile.json")
    }

    fn metafile_containing(directory: &TempDir, contents: &str) -> PathBuf {
        let metafile_path = directory.path().join("esbuild-meta.json");

        write(&metafile_path, contents).unwrap();

        metafile_path
    }

    fn build_into(out_dir: &Path, metafile_path: PathBuf) -> DashboardAssetsBuild {
        DashboardAssetsBuild {
            metafile_path,
            out_dir: Some(OsString::from(out_dir)),
        }
    }

    #[test]
    fn writes_the_dashboard_assets_and_points_cargo_at_them() {
        let out_dir = TempDir::new().unwrap();
        let mut cargo_instructions = Vec::new();
        let html_path = out_dir.path().join("dashboard_assets.html");

        build_into(out_dir.path(), dashboard_metafile_path())
            .run(&mut cargo_instructions)
            .unwrap();

        assert_eq!(
            read_to_string(&html_path).unwrap(),
            DashboardAssets::of(&EsbuildMetafile::from_str(DASHBOARD_METAFILE).unwrap())
                .unwrap()
                .html
        );
        assert_eq!(
            String::from_utf8(cargo_instructions).unwrap(),
            format!(
                "cargo::rerun-if-changed={}\ncargo::rustc-env=PADDLER_DASHBOARD_ASSETS_HTML={}\n",
                dashboard_metafile_path().display(),
                html_path.display()
            )
        );
    }

    #[test]
    fn a_missing_metafile_cannot_be_read() {
        let directory = TempDir::new().unwrap();
        let missing_metafile_path = directory.path().join("esbuild-meta.json");

        assert!(matches!(
            build_into(directory.path(), missing_metafile_path.clone()).run(Vec::new()),
            Err(DashboardAssetsError::MetafileUnreadable { path, source })
                if path == missing_metafile_path && source.kind() == ErrorKind::NotFound
        ));
    }

    #[test]
    fn a_metafile_that_is_not_json_is_malformed() {
        let directory = TempDir::new().unwrap();
        let metafile_path = metafile_containing(&directory, "not an esbuild metafile");

        assert!(matches!(
            build_into(directory.path(), metafile_path.clone()).run(Vec::new()),
            Err(DashboardAssetsError::MetafileMalformed { path, .. }) if path == metafile_path
        ));
    }

    #[test]
    fn a_metafile_without_the_dashboard_inputs_describes_no_dashboard() {
        let directory = TempDir::new().unwrap();
        let metafile_path = metafile_containing(&directory, r#"{ "outputs": {} }"#);

        assert!(matches!(
            build_into(directory.path(), metafile_path).run(Vec::new()),
            Err(DashboardAssetsError::DashboardInputMissing {
                input_path: "resources/css/page-admin-panel.css"
            })
        ));
    }

    #[test]
    fn a_build_without_an_out_dir_has_nowhere_to_write_the_dashboard_assets() {
        assert_eq!(
            DashboardAssetsBuild {
                metafile_path: dashboard_metafile_path(),
                out_dir: None,
            }
            .run(Vec::new())
            .err()
            .as_ref()
            .map(discriminant),
            Some(discriminant(&DashboardAssetsError::OutDirMissing))
        );
    }

    #[test]
    fn an_out_dir_that_does_not_exist_cannot_hold_the_dashboard_assets() {
        let directory = TempDir::new().unwrap();
        let missing_out_dir = directory.path().join("missing");

        assert!(matches!(
            build_into(&missing_out_dir, dashboard_metafile_path()).run(Vec::new()),
            Err(DashboardAssetsError::HtmlUnwritable { path, source })
                if path == missing_out_dir.join("dashboard_assets.html")
                    && source.kind() == ErrorKind::NotFound
        ));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn cargo_instructions_sent_to_a_full_device_cannot_be_written() {
        let out_dir = TempDir::new().unwrap();

        assert!(matches!(
            build_into(out_dir.path(), dashboard_metafile_path())
                .run(OpenOptions::new().write(true).open("/dev/full").unwrap()),
            Err(DashboardAssetsError::CargoInstructionsUnwritable(source))
                if source.kind() == ErrorKind::StorageFull
        ));
    }
}
