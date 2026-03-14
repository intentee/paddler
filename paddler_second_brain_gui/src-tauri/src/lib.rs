pub fn run() -> anyhow::Result<()> {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .map_err(|error| anyhow::anyhow!("failed to run tauri application: {}", error))?;

    Ok(())
}
