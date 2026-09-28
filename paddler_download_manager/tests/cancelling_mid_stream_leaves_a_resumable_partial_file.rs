use paddler_download_manager::download_outcome::DownloadOutcome;
use paddler_download_manager::download_progress::DownloadProgress;
use paddler_local_http_fixture::fixture_response::FixtureResponse;
use paddler_local_http_fixture::local_http_fixture::LocalHttpFixture;
use tempfile::TempDir;
use tokio::fs::try_exists;
use tokio::task::yield_now;
use tokio_util::sync::CancellationToken;

use crate::recorded_download::RecordedDownload;

#[tokio::test(flavor = "multi_thread")]
async fn cancelling_mid_stream_leaves_a_resumable_partial_file() {
    let directory = TempDir::new().expect("a temporary directory must be creatable");
    let destination = directory.path().join("model.gguf");
    let partial_path = destination.with_extension("partial");
    let fixture = LocalHttpFixture::start(FixtureResponse::StalledBody {
        sent_body: vec![7_u8; 16],
        withheld_byte_count: 8176,
    })
    .await
    .expect("the local HTTP fixture must start");
    let url = fixture.url("/model.gguf");
    let cancellation_token = CancellationToken::new();
    let download_cancellation_token = cancellation_token.clone();
    let download_destination = destination.clone();
    let download = tokio::spawn(async move {
        RecordedDownload::download(&download_cancellation_token, &url, &download_destination)
            .await
            .expect("the download manager must build its HTTP client")
    });

    while !try_exists(&partial_path)
        .await
        .expect("the partial path must be inspectable")
    {
        yield_now().await;
    }

    cancellation_token.cancel();

    let recorded_download = download.await.expect("the download task must not panic");

    assert_eq!(
        recorded_download
            .result
            .as_ref()
            .expect("a cancelled download must not fail"),
        &DownloadOutcome::Cancelled
    );
    assert!(
        try_exists(&partial_path)
            .await
            .expect("the partial path must be inspectable")
    );
    assert!(
        !try_exists(&destination)
            .await
            .expect("the destination must be inspectable")
    );
    assert_ne!(
        recorded_download.progress.last(),
        Some(&DownloadProgress::Finished)
    );
}
