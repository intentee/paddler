use std::path::PathBuf;

use anyhow::Result;
use anyhow::bail;

const PADDLER_GUI_BINARY_DEFAULT_PATH: &str = "../target/debug/paddler_gui";

pub fn paddler_gui_binary_path() -> Result<PathBuf> {
    let path = std::env::var("PADDLER_GUI_BINARY_PATH").map_or_else(
        |_| PathBuf::from(PADDLER_GUI_BINARY_DEFAULT_PATH),
        PathBuf::from,
    );

    if !path.exists() {
        bail!(
            "paddler_gui binary not found at {}; build it first (e.g. `cargo build -p paddler_gui`)",
            path.display()
        );
    }

    Ok(path)
}
