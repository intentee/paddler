use paddler_local_http_fixture::fixture_response::FixtureResponse;
use paddler_local_http_fixture::local_http_fixture::LocalHttpFixture;
use tempfile::TempDir;
use tokio::fs::read;
use tokio::fs::remove_dir_all;
use tokio_util::sync::CancellationToken;

use crate::recorded_download::RecordedDownload;

#[tokio::test]
async fn downloads_again_after_the_cache_directory_was_deleted() {
    let directory = TempDir::new().expect("a temporary directory must be creatable");
    let cache_directory = directory.path().join("cache");
    let destination = cache_directory.join("model.gguf");
    let body = b"model bytes for the recreation test".to_vec();
    let fixture = LocalHttpFixture::start(FixtureResponse::Ok(body.clone()))
        .await
        .expect("the local HTTP fixture must start");
    let url = fixture.url("/model.gguf");

    RecordedDownload::download(&CancellationToken::new(), &url, &destination)
        .await
        .expect("the download manager must build its HTTP client")
        .result
        .expect("the first download must succeed");
    remove_dir_all(&cache_directory)
        .await
        .expect("the cache directory must be removable");
    RecordedDownload::download(&CancellationToken::new(), &url, &destination)
        .await
        .expect("the download manager must build its HTTP client")
        .result
        .expect("the download must succeed again after the cache directory was deleted");

    assert_eq!(
        read(&destination)
            .await
            .expect("the downloaded file must be readable"),
        body
    );
    assert_eq!(fixture.request_count(), 2);
}
