use std::io;
use std::path::PathBuf;

use serde_json::Error as SerdeJsonError;
use thiserror::Error;
use url::ParseError;

#[derive(Debug, Error)]
pub enum StateDatabaseError {
    #[error("Unable to parse the state database file '{path_display}'. Either that is not a valid database file, or this version of Paddler is incompatible with it.", path_display = path.display())]
    FileContentsInvalid {
        path: PathBuf,
        #[source]
        source: SerdeJsonError,
    },
    #[error("Unable to create the state database file '{path_display}'", path_display = path.display())]
    FileCreationFailed {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("State database file path cannot be empty")]
    FilePathEmpty,
    #[error("To avoid ambiguity, needing to guess the full file path (and to stay safe overall), Paddler requires absolute paths.\nThe path you wanted is *probably* '{suggested_path_display}'. If that is so, pass it as '--state-database file://{suggested_path_display}'.", suggested_path_display = suggested_path.display())]
    FilePathRelative { suggested_path: PathBuf },
    #[error(
        "Paddler requires an absolute state database path, and the relative path '{path}' cannot be resolved"
    )]
    FilePathRelativeUnresolvable {
        path: String,
        #[source]
        source: io::Error,
    },
    #[error("Unable to read the state database file '{path_display}'", path_display = path.display())]
    FileReadFailed {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("Unable to sync the state database file '{path_display}'", path_display = path.display())]
    FileSyncFailed {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("Invalid file URL: {input}")]
    FileUrlMalformed { input: String },
    #[error("Unable to write the state database file '{path_display}'", path_display = path.display())]
    FileWriteFailed {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("Failed to serialize the state database schema")]
    SchemaUnserializable(#[source] SerdeJsonError),
    #[error("Unsupported state database scheme '{scheme}'")]
    SchemeUnsupported { scheme: String },
    #[error("Invalid state database URL '{input}'")]
    UrlInvalid {
        input: String,
        #[source]
        source: ParseError,
    },
}
