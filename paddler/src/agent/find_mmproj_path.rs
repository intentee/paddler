use std::path::PathBuf;

use log::info;

pub fn find_mmproj_path(model_path: &PathBuf) -> Option<PathBuf> {
    let model_directory = model_path.parent()?;

    let mmproj_entries: Vec<PathBuf> = std::fs::read_dir(model_directory)
        .ok()?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.contains("mmproj") && name.ends_with(".gguf"))
        })
        .collect();

    match mmproj_entries.len() {
        0 => {
            info!("No mmproj file found in {}", model_directory.display());

            None
        }
        1 => Some(mmproj_entries.into_iter().next()?),
        _ => {
            info!(
                "Multiple mmproj files found in {}, using first: {}",
                model_directory.display(),
                mmproj_entries[0].display()
            );

            mmproj_entries.into_iter().next()
        }
    }
}
