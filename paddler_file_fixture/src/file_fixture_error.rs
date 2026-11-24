use std::io;
use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum FileFixtureError {
    #[error("failed to read the fixture {}", path.display())]
    FixtureUnreadable {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
}
