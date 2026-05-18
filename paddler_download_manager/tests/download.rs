use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::time::Duration;

use anyhow::Result;
use paddler_download_manager::download_error::DownloadError;
use paddler_download_manager::download_manager::DownloadManager;
use paddler_download_manager::progress_sink::ProgressSink;
use paddler_download_manager::retry_policy::RetryPolicy;
use tempfile::TempDir;

use crate::local_http_fixture::FixtureResponse;
use crate::local_http_fixture::LocalHttpFixture;
use crate::local_http_fixture::Scenario;

mod local_http_fixture;

struct RecordingSink {
    chunk_count: AtomicU64,
    chunk_bytes: AtomicU64,
    finished_count: AtomicU64,
    started_total: AtomicU64,
    started_already: AtomicU64,
}

impl RecordingSink {
    const fn new() -> Self {
        Self {
            chunk_bytes: AtomicU64::new(0),
            chunk_count: AtomicU64::new(0),
            finished_count: AtomicU64::new(0),
            started_already: AtomicU64::new(0),
            started_total: AtomicU64::new(0),
        }
    }
}

impl ProgressSink for RecordingSink {
    fn on_started(&self, total_bytes: u64, already_downloaded: u64) {
        self.started_total.store(total_bytes, Ordering::Relaxed);
        self.started_already
            .store(already_downloaded, Ordering::Relaxed);
    }
    fn on_chunk(&self, additional_bytes: u64) {
        self.chunk_bytes
            .fetch_add(additional_bytes, Ordering::Relaxed);
        self.chunk_count.fetch_add(1, Ordering::Relaxed);
    }
    fn on_finished(&self) {
        self.finished_count.fetch_add(1, Ordering::Relaxed);
    }
}

const fn fast_retry_policy() -> RetryPolicy {
    RetryPolicy {
        initial_backoff: Duration::from_millis(1),
        max_attempts: 3,
        max_backoff: Duration::from_millis(5),
    }
}

#[tokio::test]
async fn streams_200_response_to_disk_and_calls_progress_sink_per_chunk() -> Result<()> {
    let directory = TempDir::new()?;
    let body = b"Hello, GGUF world!".to_vec();
    let fixture =
        LocalHttpFixture::start(Scenario::always(FixtureResponse::ok(body.clone()))).await?;
    let sink = Arc::new(RecordingSink::new());
    let progress_sink: Arc<dyn ProgressSink> = sink.clone();
    let dest = directory.path().join("model.gguf");

    DownloadManager::new()
        .download(&fixture.url("/model.gguf"), &dest, progress_sink)
        .await?;

    assert_eq!(tokio::fs::read(&dest).await?, body);
    assert_eq!(
        sink.started_total.load(Ordering::Relaxed),
        body.len() as u64
    );
    assert_eq!(sink.started_already.load(Ordering::Relaxed), 0);
    assert_eq!(sink.chunk_bytes.load(Ordering::Relaxed), body.len() as u64);
    assert!(sink.chunk_count.load(Ordering::Relaxed) >= 1);
    assert_eq!(sink.finished_count.load(Ordering::Relaxed), 1);

    Ok(())
}

#[tokio::test]
async fn resumes_from_existing_partial_file_with_range_request() -> Result<()> {
    let directory = TempDir::new()?;
    let dest = directory.path().join("model.gguf");
    let partial_path = dest.with_extension("partial");
    tokio::fs::write(&partial_path, b"first half ").await?;

    let body = b"second half".to_vec();
    let fixture = LocalHttpFixture::start(Scenario::always(FixtureResponse::partial_content(
        body.clone(),
    )))
    .await?;
    let sink = Arc::new(RecordingSink::new());
    let progress_sink: Arc<dyn ProgressSink> = sink.clone();

    DownloadManager::new()
        .download(&fixture.url("/model.gguf"), &dest, progress_sink)
        .await?;

    assert_eq!(tokio::fs::read(&dest).await?, b"first half second half");
    assert_eq!(sink.started_already.load(Ordering::Relaxed), 11);
    assert!(
        fixture
            .last_recorded_range_header()
            .unwrap_or_default()
            .contains("bytes=11-")
    );

    Ok(())
}

#[tokio::test]
async fn starts_over_when_server_returns_200_to_range_request() -> Result<()> {
    let directory = TempDir::new()?;
    let dest = directory.path().join("model.gguf");
    let partial_path = dest.with_extension("partial");
    tokio::fs::write(&partial_path, b"stale partial bytes").await?;

    let body = b"fresh entire body".to_vec();
    let fixture =
        LocalHttpFixture::start(Scenario::always(FixtureResponse::ok(body.clone()))).await?;
    let sink = Arc::new(RecordingSink::new());
    let progress_sink: Arc<dyn ProgressSink> = sink.clone();

    DownloadManager::new()
        .download(&fixture.url("/model.gguf"), &dest, progress_sink)
        .await?;

    assert_eq!(tokio::fs::read(&dest).await?, body);

    Ok(())
}

#[tokio::test]
async fn returns_not_found_on_404_without_retrying() -> Result<()> {
    let directory = TempDir::new()?;
    let dest = directory.path().join("model.gguf");
    let fixture = LocalHttpFixture::start(Scenario::always(FixtureResponse::status(404))).await?;
    let sink: Arc<dyn ProgressSink> = Arc::new(RecordingSink::new());

    let result = DownloadManager::with_policy(fast_retry_policy())
        .download(&fixture.url("/missing.gguf"), &dest, sink)
        .await;

    assert!(matches!(result, Err(DownloadError::NotFound { .. })));
    assert_eq!(fixture.request_count(), 1);

    Ok(())
}

#[tokio::test]
async fn returns_permission_denied_on_401_without_retrying() -> Result<()> {
    let directory = TempDir::new()?;
    let dest = directory.path().join("model.gguf");
    let fixture = LocalHttpFixture::start(Scenario::always(FixtureResponse::status(401))).await?;
    let sink: Arc<dyn ProgressSink> = Arc::new(RecordingSink::new());

    let result = DownloadManager::with_policy(fast_retry_policy())
        .download(&fixture.url("/private.gguf"), &dest, sink)
        .await;

    assert!(matches!(
        result,
        Err(DownloadError::PermissionDenied { .. })
    ));
    assert_eq!(fixture.request_count(), 1);

    Ok(())
}

#[tokio::test]
async fn returns_permission_denied_on_403_without_retrying() -> Result<()> {
    let directory = TempDir::new()?;
    let dest = directory.path().join("model.gguf");
    let fixture = LocalHttpFixture::start(Scenario::always(FixtureResponse::status(403))).await?;
    let sink: Arc<dyn ProgressSink> = Arc::new(RecordingSink::new());

    let result = DownloadManager::with_policy(fast_retry_policy())
        .download(&fixture.url("/forbidden.gguf"), &dest, sink)
        .await;

    assert!(matches!(
        result,
        Err(DownloadError::PermissionDenied { .. })
    ));
    assert_eq!(fixture.request_count(), 1);

    Ok(())
}

#[tokio::test]
async fn returns_partial_file_stale_on_416_and_removes_partial() -> Result<()> {
    let directory = TempDir::new()?;
    let dest = directory.path().join("model.gguf");
    let partial_path = dest.with_extension("partial");
    tokio::fs::write(&partial_path, b"stale").await?;

    let fixture = LocalHttpFixture::start(Scenario::always(FixtureResponse::status(416))).await?;
    let sink: Arc<dyn ProgressSink> = Arc::new(RecordingSink::new());

    let result = DownloadManager::with_policy(fast_retry_policy())
        .download(&fixture.url("/model.gguf"), &dest, sink)
        .await;

    assert!(matches!(
        result,
        Err(DownloadError::PartialFileStale { .. })
    ));
    assert!(!tokio::fs::try_exists(&partial_path).await?);

    Ok(())
}

#[tokio::test]
async fn returns_partial_file_stale_on_416_even_when_no_partial_existed() -> Result<()> {
    let directory = TempDir::new()?;
    let dest = directory.path().join("model.gguf");
    let partial_path = dest.with_extension("partial");

    let fixture = LocalHttpFixture::start(Scenario::always(FixtureResponse::status(416))).await?;
    let sink: Arc<dyn ProgressSink> = Arc::new(RecordingSink::new());

    let result = DownloadManager::with_policy(fast_retry_policy())
        .download(&fixture.url("/model.gguf"), &dest, sink)
        .await;

    assert!(matches!(
        result,
        Err(DownloadError::PartialFileStale { .. })
    ));
    assert!(!tokio::fs::try_exists(&partial_path).await?);

    Ok(())
}

#[tokio::test]
async fn retries_on_503_then_succeeds() -> Result<()> {
    let directory = TempDir::new()?;
    let dest = directory.path().join("model.gguf");
    let body = b"recovered".to_vec();
    let fixture = LocalHttpFixture::start(Scenario::sequence(vec![
        FixtureResponse::status(503),
        FixtureResponse::ok(body.clone()),
    ]))
    .await?;
    let sink: Arc<dyn ProgressSink> = Arc::new(RecordingSink::new());

    DownloadManager::with_policy(fast_retry_policy())
        .download(&fixture.url("/model.gguf"), &dest, sink)
        .await?;

    assert_eq!(tokio::fs::read(&dest).await?, body);
    assert_eq!(fixture.request_count(), 2);

    Ok(())
}

#[tokio::test]
async fn retries_on_500_until_exhausted_returns_network_exhausted() -> Result<()> {
    let directory = TempDir::new()?;
    let dest = directory.path().join("model.gguf");
    let fixture = LocalHttpFixture::start(Scenario::always(FixtureResponse::status(500))).await?;
    let sink: Arc<dyn ProgressSink> = Arc::new(RecordingSink::new());

    let result = DownloadManager::with_policy(fast_retry_policy())
        .download(&fixture.url("/model.gguf"), &dest, sink)
        .await;

    assert!(matches!(
        result,
        Err(DownloadError::NetworkExhausted { attempts: 3, .. })
    ));
    assert_eq!(fixture.request_count(), 3);

    Ok(())
}

#[tokio::test]
async fn resumes_intra_call_when_connection_drops_mid_stream() -> Result<()> {
    let directory = TempDir::new()?;
    let dest = directory.path().join("model.gguf");
    let full_body = b"abcdefghijklmnop".to_vec();
    let fixture = LocalHttpFixture::start(Scenario::sequence(vec![
        FixtureResponse::ok_drop_after(full_body.clone(), 6),
        FixtureResponse::partial_content(full_body[6..].to_vec()),
    ]))
    .await?;
    let sink = Arc::new(RecordingSink::new());
    let progress_sink: Arc<dyn ProgressSink> = sink.clone();

    DownloadManager::with_policy(fast_retry_policy())
        .download(&fixture.url("/model.gguf"), &dest, progress_sink)
        .await?;

    assert_eq!(tokio::fs::read(&dest).await?, full_body);
    assert_eq!(fixture.request_count(), 2);

    Ok(())
}

#[tokio::test]
async fn progress_sink_on_finished_fires_only_on_success() -> Result<()> {
    let directory = TempDir::new()?;
    let dest = directory.path().join("model.gguf");

    let fixture_success =
        LocalHttpFixture::start(Scenario::always(FixtureResponse::ok(b"ok body".to_vec()))).await?;
    let sink_success = Arc::new(RecordingSink::new());
    let progress_success: Arc<dyn ProgressSink> = sink_success.clone();
    DownloadManager::with_policy(fast_retry_policy())
        .download(&fixture_success.url("/x"), &dest, progress_success)
        .await?;
    assert_eq!(sink_success.finished_count.load(Ordering::Relaxed), 1);

    tokio::fs::remove_file(&dest).await?;

    let fixture_404 =
        LocalHttpFixture::start(Scenario::always(FixtureResponse::status(404))).await?;
    let sink_404 = Arc::new(RecordingSink::new());
    let progress_404: Arc<dyn ProgressSink> = sink_404.clone();
    let _ = DownloadManager::with_policy(fast_retry_policy())
        .download(&fixture_404.url("/x"), &dest, progress_404)
        .await;
    assert_eq!(sink_404.finished_count.load(Ordering::Relaxed), 0);

    let fixture_500 =
        LocalHttpFixture::start(Scenario::always(FixtureResponse::status(500))).await?;
    let sink_500 = Arc::new(RecordingSink::new());
    let progress_500: Arc<dyn ProgressSink> = sink_500.clone();
    let _ = DownloadManager::with_policy(fast_retry_policy())
        .download(&fixture_500.url("/x"), &dest, progress_500)
        .await;
    assert_eq!(sink_500.finished_count.load(Ordering::Relaxed), 0);

    Ok(())
}

#[tokio::test]
async fn invalid_url_returns_invalid_url_error_without_network_call() -> Result<()> {
    let directory = TempDir::new()?;
    let dest = directory.path().join("model.gguf");
    let sink: Arc<dyn ProgressSink> = Arc::new(RecordingSink::new());

    let result = DownloadManager::with_policy(fast_retry_policy())
        .download("not a valid url", &dest, sink)
        .await;

    assert!(matches!(result, Err(DownloadError::InvalidUrl { .. })));

    Ok(())
}

#[tokio::test]
async fn fixture_serves_configured_status_and_body() -> Result<()> {
    let fixture =
        LocalHttpFixture::start(Scenario::always(FixtureResponse::ok(b"hello".to_vec()))).await?;
    let response = reqwest::get(fixture.url("/x")).await?;

    assert_eq!(response.status(), 200);
    assert_eq!(response.bytes().await?.as_ref(), b"hello");
    assert_eq!(fixture.request_count(), 1);

    Ok(())
}

#[tokio::test]
async fn fixture_distinct_ports_per_instance() -> Result<()> {
    let first = LocalHttpFixture::start(Scenario::always(FixtureResponse::ok(Vec::new()))).await?;
    let second = LocalHttpFixture::start(Scenario::always(FixtureResponse::ok(Vec::new()))).await?;

    assert_ne!(first.port(), second.port());

    Ok(())
}

#[tokio::test]
async fn fixture_drops_connection_when_configured_to() -> Result<()> {
    let fixture = LocalHttpFixture::start(Scenario::always(FixtureResponse::ok_drop_after(
        b"abcdefgh".to_vec(),
        4,
    )))
    .await?;
    let response = reqwest::get(fixture.url("/x")).await?;
    let body_result = response.bytes().await;

    assert!(body_result.is_err(), "expected dropped connection during body read");

    Ok(())
}

#[tokio::test]
async fn fixture_request_count_increments() -> Result<()> {
    let fixture =
        LocalHttpFixture::start(Scenario::always(FixtureResponse::ok(Vec::new()))).await?;

    let _ = reqwest::get(fixture.url("/a")).await?;
    let _ = reqwest::get(fixture.url("/b")).await?;
    let _ = reqwest::get(fixture.url("/c")).await?;

    assert_eq!(fixture.request_count(), 3);

    Ok(())
}

#[tokio::test]
async fn returns_io_error_when_destination_directory_does_not_exist_and_cannot_be_created()
-> Result<()> {
    // Pointing the destination at a path under an existing FILE (not directory) — create_dir_all will fail
    // because a regular file blocks the parent directory creation.
    let directory = TempDir::new()?;
    let blocker = directory.path().join("blocker");
    tokio::fs::write(&blocker, b"i am a file, not a directory").await?;
    let dest = blocker.join("subdir").join("model.gguf");

    let fixture =
        LocalHttpFixture::start(Scenario::always(FixtureResponse::ok(b"body".to_vec()))).await?;
    let sink: Arc<dyn ProgressSink> = Arc::new(RecordingSink::new());

    let result = DownloadManager::with_policy(fast_retry_policy())
        .download(&fixture.url("/x"), &dest, sink)
        .await;

    assert!(matches!(result, Err(DownloadError::Io { .. })));

    Ok(())
}

#[tokio::test]
async fn last_recorded_range_header_returns_none_when_no_range_was_sent() -> Result<()> {
    let directory = TempDir::new()?;
    let dest = directory.path().join("model.gguf");
    let fixture =
        LocalHttpFixture::start(Scenario::always(FixtureResponse::ok(b"body".to_vec()))).await?;
    let sink: Arc<dyn ProgressSink> = Arc::new(RecordingSink::new());

    DownloadManager::new()
        .download(&fixture.url("/x"), &dest, sink)
        .await?;

    assert!(fixture.last_recorded_range_header().is_none());

    Ok(())
}

#[tokio::test]
async fn send_error_treated_as_transient_then_exhausted() -> Result<()> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let port = listener.local_addr()?.port();
    drop(listener);

    let directory = TempDir::new()?;
    let dest = directory.path().join("model.gguf");
    let sink: Arc<dyn ProgressSink> = Arc::new(RecordingSink::new());

    let result = DownloadManager::with_policy(fast_retry_policy())
        .download(
            &format!("http://127.0.0.1:{port}/never-listens"),
            &dest,
            sink,
        )
        .await;

    assert!(matches!(
        result,
        Err(DownloadError::NetworkExhausted { attempts: 3, .. })
    ));

    Ok(())
}

#[cfg(unix)]
#[tokio::test]
async fn open_for_append_error_returns_io_when_partial_path_is_a_directory() -> Result<()> {
    let directory = TempDir::new()?;
    let dest = directory.path().join("model.gguf");
    let partial_path = dest.with_extension("partial");
    tokio::fs::create_dir(&partial_path).await?;

    let fixture =
        LocalHttpFixture::start(Scenario::always(FixtureResponse::ok(b"body".to_vec()))).await?;
    let sink: Arc<dyn ProgressSink> = Arc::new(RecordingSink::new());

    let result = DownloadManager::with_policy(fast_retry_policy())
        .download(&fixture.url("/x"), &dest, sink)
        .await;

    assert!(matches!(result, Err(DownloadError::Io { .. })));

    Ok(())
}

#[cfg(unix)]
#[tokio::test]
async fn open_for_append_error_returns_io_when_parent_is_read_only() -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let directory = TempDir::new()?;
    let readonly_parent = directory.path().join("readonly");
    tokio::fs::create_dir(&readonly_parent).await?;
    let dest = readonly_parent.join("model.gguf");
    let mut perms = tokio::fs::metadata(&readonly_parent).await?.permissions();
    perms.set_mode(0o500);
    tokio::fs::set_permissions(&readonly_parent, perms).await?;

    let fixture =
        LocalHttpFixture::start(Scenario::always(FixtureResponse::ok(b"body".to_vec()))).await?;
    let sink: Arc<dyn ProgressSink> = Arc::new(RecordingSink::new());

    let result = DownloadManager::with_policy(fast_retry_policy())
        .download(&fixture.url("/x"), &dest, sink)
        .await;

    let mut restore = tokio::fs::metadata(&readonly_parent).await?.permissions();
    restore.set_mode(0o700);
    tokio::fs::set_permissions(&readonly_parent, restore).await?;

    assert!(matches!(result, Err(DownloadError::Io { .. })));

    Ok(())
}

#[cfg(unix)]
#[tokio::test]
async fn finalize_error_returns_io_when_destination_is_a_non_empty_directory() -> Result<()> {
    let directory = TempDir::new()?;
    let dest = directory.path().join("model.gguf");
    tokio::fs::create_dir(&dest).await?;
    tokio::fs::write(dest.join("blocker"), b"x").await?;

    let fixture =
        LocalHttpFixture::start(Scenario::always(FixtureResponse::ok(b"body".to_vec()))).await?;
    let sink: Arc<dyn ProgressSink> = Arc::new(RecordingSink::new());

    let result = DownloadManager::with_policy(fast_retry_policy())
        .download(&fixture.url("/x"), &dest, sink)
        .await;

    assert!(matches!(result, Err(DownloadError::Io { .. })));

    Ok(())
}

#[cfg(unix)]
#[tokio::test]
async fn partial_file_stale_with_unremovable_partial_returns_io_error() -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let directory = TempDir::new()?;
    let locked_parent = directory.path().join("locked");
    tokio::fs::create_dir(&locked_parent).await?;
    let dest = locked_parent.join("model.gguf");
    let partial_path = dest.with_extension("partial");
    tokio::fs::write(&partial_path, b"stale").await?;
    let mut perms = tokio::fs::metadata(&locked_parent).await?.permissions();
    perms.set_mode(0o500);
    tokio::fs::set_permissions(&locked_parent, perms).await?;

    let fixture = LocalHttpFixture::start(Scenario::always(FixtureResponse::status(416))).await?;
    let sink: Arc<dyn ProgressSink> = Arc::new(RecordingSink::new());

    let result = DownloadManager::with_policy(fast_retry_policy())
        .download(&fixture.url("/x"), &dest, sink)
        .await;

    let mut restore = tokio::fs::metadata(&locked_parent).await?.permissions();
    restore.set_mode(0o700);
    tokio::fs::set_permissions(&locked_parent, restore).await?;

    assert!(matches!(result, Err(DownloadError::Io { .. })));

    Ok(())
}

#[cfg(unix)]
#[tokio::test]
async fn truncate_error_during_ignore_range_returns_io() -> Result<()> {
    let directory = TempDir::new()?;
    let dest = directory.path().join("model.gguf");
    let partial_path = dest.with_extension("partial");
    tokio::fs::create_dir(&partial_path).await?;

    let fixture = LocalHttpFixture::start(Scenario::always(FixtureResponse::ok(b"body".to_vec())))
        .await?;
    let sink: Arc<dyn ProgressSink> = Arc::new(RecordingSink::new());

    let result = DownloadManager::with_policy(fast_retry_policy())
        .download(&fixture.url("/x"), &dest, sink)
        .await;

    assert!(matches!(result, Err(DownloadError::Io { .. })));

    Ok(())
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn stream_write_failure_via_dev_full_returns_io_error() -> Result<()> {
    let directory = TempDir::new()?;
    let dest = directory.path().join("model.gguf");
    let partial_path = dest.with_extension("partial");
    std::os::unix::fs::symlink("/dev/full", &partial_path)?;

    let fixture = LocalHttpFixture::start(Scenario::always(FixtureResponse::ok(
        b"this body will fail to write because /dev/full".to_vec(),
    )))
    .await?;
    let sink: Arc<dyn ProgressSink> = Arc::new(RecordingSink::new());

    let result = DownloadManager::with_policy(fast_retry_policy())
        .download(&fixture.url("/x"), &dest, sink)
        .await;

    assert!(matches!(result, Err(DownloadError::Io { .. })));

    Ok(())
}
