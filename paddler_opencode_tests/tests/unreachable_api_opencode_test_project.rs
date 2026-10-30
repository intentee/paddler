use url::Url;

use paddler_opencode_tests::opencode_test_project::OpenCodeTestProject;

pub fn unreachable_api_opencode_test_project() -> OpenCodeTestProject {
    OpenCodeTestProject::create(
        &Url::parse("http://127.0.0.1:1/v1").expect("the unreachable API URL must be valid"),
        "marker".to_owned(),
    )
    .expect("an OpenCode test project must be creatable")
}
