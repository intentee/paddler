use anyhow::Result;
use paddler_tests::state_database_file::StateDatabaseFile;

#[test]
fn harness_state_database_file_builds_file_url() -> Result<()> {
    let database = StateDatabaseFile::new()?;

    assert!(
        database.url.starts_with("file://"),
        "expected URL to start with file://; got {url:?}",
        url = database.url
    );

    Ok(())
}
