use std::env::VarError;
use std::io::Error as IoError;
use std::path::PathBuf;

use serde_json::Error as SerdeJsonError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum OpenCodeTestError {
    #[error("environment variable {variable} does not provide the OpenCode binary path")]
    BinaryPathNotProvided {
        variable: String,
        #[source]
        source: VarError,
    },

    #[error("the OpenCode binary does not exist at {path}")]
    BinaryDoesNotExist { path: PathBuf },

    #[error("failed to set up the OpenCode test project")]
    ProjectSetupFailed {
        #[source]
        source: IoError,
    },

    #[error("failed to serialize the OpenCode config")]
    ConfigSerializationFailed {
        #[source]
        source: SerdeJsonError,
    },

    #[error("failed to spawn the OpenCode binary at {path}")]
    SpawnFailed {
        path: PathBuf,
        #[source]
        source: IoError,
    },

    #[error("failed to wait for the OpenCode process to finish")]
    ProcessWaitFailed {
        #[source]
        source: IoError,
    },
}
