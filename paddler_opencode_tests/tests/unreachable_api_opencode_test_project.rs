use anyhow::Result;
use url::Url;

use paddler_opencode_tests::opencode_test_project::OpenCodeTestProject;

pub fn unreachable_api_opencode_test_project() -> Result<OpenCodeTestProject> {
    let api_base_url = Url::parse("http://127.0.0.1:1/v1")?;

    Ok(OpenCodeTestProject::create(
        &api_base_url,
        "marker".to_owned(),
    )?)
}
