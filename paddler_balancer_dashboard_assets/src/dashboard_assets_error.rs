use std::io;
use std::path::PathBuf;

use esbuild_metafile::error::Error as EsbuildMetafileError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DashboardAssetsError {
    #[error("Unable to write the Cargo instructions of the dashboard assets build")]
    CargoInstructionsUnwritable(#[source] io::Error),
    #[error("The esbuild metafile does not describe the dashboard input '{input_path}'")]
    DashboardInputMissing { input_path: &'static str },
    #[error("Unable to write the dashboard assets to '{path_display}'", path_display = path.display())]
    HtmlUnwritable {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("The esbuild metafile '{path_display}' is not an esbuild metafile", path_display = path.display())]
    MetafileMalformed {
        path: PathBuf,
        #[source]
        source: EsbuildMetafileError,
    },
    #[error("Unable to read the esbuild metafile '{path_display}'", path_display = path.display())]
    MetafileUnreadable {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("Cargo did not provide the OUT_DIR to write the dashboard assets to")]
    OutDirMissing,
}
