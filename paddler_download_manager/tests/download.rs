use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

use anyhow::Result;
use anyhow::anyhow;
use anyhow::bail;
use paddler_download_manager::download_error::DownloadError;
use paddler_download_manager::download_manager::DownloadManager;
use paddler_download_manager::progress_sink::ProgressSink;

use tempfile::TempDir;
use tokio::fs::create_dir;
use tokio::fs::metadata;
use tokio::fs::read;
use tokio::fs::remove_dir_all;
use tokio::fs::remove_file;
use tokio::fs::set_permissions;
use tokio::fs::try_exists;
use tokio::fs::write;

use crate::local_http_fixture::FixtureResponse;
use crate::local_http_fixture::LocalHttpFixture;
use crate::local_http_fixture::Scenario;

struct RecordingSink {
    chunk_count: AtomicU64,
    chunk_bytes: AtomicU64,
    finished_count: AtomicU64,
    started_total: AtomicU64,
    started_total_indeterminate: AtomicBool,
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
            started_total_indeterminate: AtomicBool::new(true),
        }
    }
}

impl ProgressSink for RecordingSink {
    fn on_started(&self, total_bytes: Option<u64>, already_downloaded: u64) {
        match total_bytes {
            Some(value) => {
                self.started_total.store(value, Ordering::Relaxed);
                self.started_total_indeterminate
                    .store(false, Ordering::Relaxed);
            }
            None => {
                self.started_total_indeterminate
                    .store(true, Ordering::Relaxed);
            }
        }
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

#[tokio::test]
async fn streams_200_response_to_disk_and_calls_progress_sink_per_chunk() -> Result<()> {
    let directory = TempDir::new()?;
    let body = b"Hello, GGUF world!".to_vec();
    let fixture =
        LocalHttpFixture::start(Scenario::always(FixtureResponse::ok(body.clone()))).await?;
    let sink = Arc::new(RecordingSink::new());
    let progress_sink: Arc<dyn ProgressSink> = sink.clone();
    let dest = directory.path().join("model.gguf");

    DownloadManager::new()?
        .download(&fixture.url("/model.gguf"), &dest, progress_sink)
        .await?;

    assert_eq!(read(&dest).await?, body);
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
    write(&partial_path, b"first half ").await?;

    let body = b"second half".to_vec();
    let total = 11_u64 + body.len() as u64;
    let fixture = LocalHttpFixture::start(Scenario::always(
        FixtureResponse::partial_content_with_range(
            body.clone(),
            format!("bytes 11-{}/{}", total - 1, total),
        ),
    ))
    .await?;
    let sink = Arc::new(RecordingSink::new());
    let progress_sink: Arc<dyn ProgressSink> = sink.clone();

    DownloadManager::new()?
        .download(&fixture.url("/model.gguf"), &dest, progress_sink)
        .await?;

    assert_eq!(read(&dest).await?, b"first half second half");
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
    write(&partial_path, b"stale partial bytes").await?;

    let body = b"fresh entire body".to_vec();
    let fixture =
        LocalHttpFixture::start(Scenario::always(FixtureResponse::ok(body.clone()))).await?;
    let sink = Arc::new(RecordingSink::new());
    let progress_sink: Arc<dyn ProgressSink> = sink.clone();

    DownloadManager::new()?
        .download(&fixture.url("/model.gguf"), &dest, progress_sink)
        .await?;

    assert_eq!(read(&dest).await?, body);

    Ok(())
}

#[tokio::test]
async fn returns_not_found_on_404_without_retrying() -> Result<()> {
    let directory = TempDir::new()?;
    let dest = directory.path().join("model.gguf");
    let fixture = LocalHttpFixture::start(Scenario::always(FixtureResponse::status(404))).await?;
    let sink: Arc<dyn ProgressSink> = Arc::new(RecordingSink::new());

    let result = DownloadManager::new()?
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

    let result = DownloadManager::new()?
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

    let result = DownloadManager::new()?
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
    write(&partial_path, b"stale").await?;

    let fixture = LocalHttpFixture::start(Scenario::always(FixtureResponse::status(416))).await?;
    let sink: Arc<dyn ProgressSink> = Arc::new(RecordingSink::new());

    let result = DownloadManager::new()?
        .download(&fixture.url("/model.gguf"), &dest, sink)
        .await;

    assert!(matches!(
        result,
        Err(DownloadError::PartialFileStale { .. })
    ));
    assert!(!try_exists(&partial_path).await?);

    Ok(())
}

#[tokio::test]
async fn returns_partial_file_stale_on_416_even_when_no_partial_existed() -> Result<()> {
    let directory = TempDir::new()?;
    let dest = directory.path().join("model.gguf");
    let partial_path = dest.with_extension("partial");

    let fixture = LocalHttpFixture::start(Scenario::always(FixtureResponse::status(416))).await?;
    let sink: Arc<dyn ProgressSink> = Arc::new(RecordingSink::new());

    let result = DownloadManager::new()?
        .download(&fixture.url("/model.gguf"), &dest, sink)
        .await;

    assert!(matches!(
        result,
        Err(DownloadError::PartialFileStale { .. })
    ));
    assert!(!try_exists(&partial_path).await?);

    Ok(())
}

#[tokio::test]
async fn mismatched_content_range_is_treated_as_partial_file_stale() -> Result<()> {
    let directory = TempDir::new()?;
    let dest = directory.path().join("model.gguf");
    let partial_path = dest.with_extension("partial");
    write(&partial_path, b"first half ").await?;

    let body = b"second half".to_vec();
    let total = 11_u64 + body.len() as u64;
    let fixture = LocalHttpFixture::start(Scenario::always(
        FixtureResponse::partial_content_with_range(
            body,
            format!("bytes 999-{}/{}", 999 + 10, total),
        ),
    ))
    .await?;
    let sink: Arc<dyn ProgressSink> = Arc::new(RecordingSink::new());

    let result = DownloadManager::new()?
        .download(&fixture.url("/model.gguf"), &dest, sink)
        .await;

    assert!(matches!(
        result,
        Err(DownloadError::PartialFileStale { .. })
    ));
    assert!(!try_exists(&partial_path).await?);

    Ok(())
}

#[tokio::test]
async fn four_hundred_status_returns_download_server_rejected_request() -> Result<()> {
    let directory = TempDir::new()?;
    let dest = directory.path().join("model.gguf");
    let fixture = LocalHttpFixture::start(Scenario::always(FixtureResponse::status(400))).await?;
    let sink: Arc<dyn ProgressSink> = Arc::new(RecordingSink::new());

    let result = DownloadManager::new()?
        .download(&fixture.url("/model.gguf"), &dest, sink)
        .await;

    let Err(DownloadError::DownloadServerRejectedRequest { status, .. }) = result else {
        bail!("expected DownloadServerRejectedRequest, got {result:?}");
    };
    assert_eq!(status, reqwest::StatusCode::BAD_REQUEST);
    assert_eq!(fixture.request_count(), 1);

    Ok(())
}

#[tokio::test]
async fn five_hundred_status_returns_download_server_errored() -> Result<()> {
    let directory = TempDir::new()?;
    let dest = directory.path().join("model.gguf");
    let fixture = LocalHttpFixture::start(Scenario::always(FixtureResponse::status(500))).await?;
    let sink: Arc<dyn ProgressSink> = Arc::new(RecordingSink::new());

    let result = DownloadManager::new()?
        .download(&fixture.url("/model.gguf"), &dest, sink)
        .await;

    let Err(DownloadError::DownloadServerErrored { status, .. }) = result else {
        bail!("expected DownloadServerErrored, got {result:?}");
    };
    assert_eq!(status, reqwest::StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(fixture.request_count(), 1);

    Ok(())
}

#[tokio::test]
async fn stream_drop_after_partial_body_returns_download_stream_interrupted() -> Result<()> {
    let directory = TempDir::new()?;
    let dest = directory.path().join("model.gguf");
    let full_body = b"abcdefghijklmnop".to_vec();
    let fixture = LocalHttpFixture::start(Scenario::always(FixtureResponse::ok_drop_after(
        full_body.clone(),
        6,
    )))
    .await?;
    let sink: Arc<dyn ProgressSink> = Arc::new(RecordingSink::new());

    let result = DownloadManager::new()?
        .download(&fixture.url("/model.gguf"), &dest, sink)
        .await;

    assert!(matches!(
        result,
        Err(DownloadError::DownloadInterrupted { .. })
    ));
    assert_eq!(fixture.request_count(), 1);

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
    DownloadManager::new()?
        .download(&fixture_success.url("/x"), &dest, progress_success)
        .await?;
    assert_eq!(sink_success.finished_count.load(Ordering::Relaxed), 1);

    remove_file(&dest).await?;

    let fixture_404 =
        LocalHttpFixture::start(Scenario::always(FixtureResponse::status(404))).await?;
    let sink_404 = Arc::new(RecordingSink::new());
    let progress_404: Arc<dyn ProgressSink> = sink_404.clone();
    let _ = DownloadManager::new()?
        .download(&fixture_404.url("/x"), &dest, progress_404)
        .await;
    assert_eq!(sink_404.finished_count.load(Ordering::Relaxed), 0);

    let fixture_500 =
        LocalHttpFixture::start(Scenario::always(FixtureResponse::status(500))).await?;
    let sink_500 = Arc::new(RecordingSink::new());
    let progress_500: Arc<dyn ProgressSink> = sink_500.clone();
    let _ = DownloadManager::new()?
        .download(&fixture_500.url("/x"), &dest, progress_500)
        .await;
    assert_eq!(sink_500.finished_count.load(Ordering::Relaxed), 0);

    Ok(())
}

#[tokio::test]
async fn unsupported_url_scheme_returns_invalid_url_error_without_network_call() -> Result<()> {
    let directory = TempDir::new()?;
    let dest = directory.path().join("model.gguf");
    let sink: Arc<dyn ProgressSink> = Arc::new(RecordingSink::new());

    let result = DownloadManager::new()?
        .download("ftp://example.invalid/model.gguf", &dest, sink)
        .await;

    assert!(matches!(
        result,
        Err(DownloadError::UnsupportedUrlScheme { .. })
    ));

    Ok(())
}

#[tokio::test]
async fn invalid_url_returns_invalid_url_error_without_network_call() -> Result<()> {
    let directory = TempDir::new()?;
    let dest = directory.path().join("model.gguf");
    let sink: Arc<dyn ProgressSink> = Arc::new(RecordingSink::new());

    let result = DownloadManager::new()?
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

    assert!(
        body_result.is_err(),
        "expected dropped connection during body read"
    );

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
    let directory = TempDir::new()?;
    let blocker = directory.path().join("blocker");
    write(&blocker, b"i am a file, not a directory").await?;
    let dest = blocker.join("subdir").join("model.gguf");

    let fixture =
        LocalHttpFixture::start(Scenario::always(FixtureResponse::ok(b"body".to_vec()))).await?;
    let sink: Arc<dyn ProgressSink> = Arc::new(RecordingSink::new());

    let result = DownloadManager::new()?
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

    DownloadManager::new()?
        .download(&fixture.url("/x"), &dest, sink)
        .await?;

    assert!(fixture.last_recorded_range_header().is_none());

    Ok(())
}

#[tokio::test]
async fn read_timeout_fires_when_server_stalls_before_headers() -> Result<()> {
    use std::time::Duration;

    let directory = TempDir::new()?;
    let dest = directory.path().join("model.gguf");
    let fixture =
        LocalHttpFixture::start(Scenario::always(FixtureResponse::stall_before_headers())).await?;
    let sink: Arc<dyn ProgressSink> = Arc::new(RecordingSink::new());

    let outcome = tokio::time::timeout(
        Duration::from_secs(20),
        DownloadManager::new()?.download(&fixture.url("/model.gguf"), &dest, sink),
    )
    .await;

    let result = outcome.map_err(|_elapsed| {
        anyhow!("download did not return within test guard; read_timeout never fired")
    })?;

    assert!(
        result.is_err(),
        "stalled server must produce an error, got Ok"
    );

    Ok(())
}

#[tokio::test]
async fn send_error_returns_download_server_is_unreachable() -> Result<()> {
    let directory = TempDir::new()?;
    let dest = directory.path().join("model.gguf");
    let sink: Arc<dyn ProgressSink> = Arc::new(RecordingSink::new());

    let url = "http://127.0.0.1:1/never-listens".to_owned();
    let result = DownloadManager::new()?.download(&url, &dest, sink).await;

    let Err(DownloadError::DownloadServerIsUnreachable { url: error_url, .. }) = result else {
        bail!("expected DownloadServerIsUnreachable, got {result:?}");
    };
    assert_eq!(error_url, url);

    Ok(())
}

#[cfg(unix)]
#[tokio::test]
async fn open_for_append_error_returns_io_when_partial_path_is_a_directory() -> Result<()> {
    let directory = TempDir::new()?;
    let dest = directory.path().join("model.gguf");
    let partial_path = dest.with_extension("partial");
    create_dir(&partial_path).await?;

    let fixture =
        LocalHttpFixture::start(Scenario::always(FixtureResponse::ok(b"body".to_vec()))).await?;
    let sink: Arc<dyn ProgressSink> = Arc::new(RecordingSink::new());

    let result = DownloadManager::new()?
        .download(&fixture.url("/x"), &dest, sink)
        .await;

    assert!(matches!(result, Err(DownloadError::Io { .. })));

    Ok(())
}

#[cfg(unix)]
#[tokio::test]
async fn download_returns_cache_permission_denied_when_dir_is_read_only() -> Result<()> {
    use std::io;
    use std::os::unix::fs::PermissionsExt;

    let directory = TempDir::new()?;
    let readonly_parent = directory.path().join("readonly");
    create_dir(&readonly_parent).await?;
    let dest = readonly_parent.join("model.gguf");
    let mut perms = metadata(&readonly_parent).await?.permissions();
    perms.set_mode(0o500);
    set_permissions(&readonly_parent, perms).await?;

    let fixture =
        LocalHttpFixture::start(Scenario::always(FixtureResponse::ok(b"body".to_vec()))).await?;
    let sink: Arc<dyn ProgressSink> = Arc::new(RecordingSink::new());

    let result = DownloadManager::new()?
        .download(&fixture.url("/x"), &dest, sink)
        .await;

    let mut restore = metadata(&readonly_parent).await?.permissions();
    restore.set_mode(0o700);
    set_permissions(&readonly_parent, restore).await?;

    let Err(DownloadError::CachePermissionDenied { source, .. }) = result else {
        bail!("expected CachePermissionDenied, got {result:?}");
    };
    assert_eq!(source.kind(), io::ErrorKind::PermissionDenied);

    Ok(())
}

#[cfg(unix)]
#[tokio::test]
async fn finalize_error_returns_io_when_destination_is_a_non_empty_directory() -> Result<()> {
    let directory = TempDir::new()?;
    let dest = directory.path().join("model.gguf");
    create_dir(&dest).await?;
    write(dest.join("blocker"), b"x").await?;

    let fixture =
        LocalHttpFixture::start(Scenario::always(FixtureResponse::ok(b"body".to_vec()))).await?;
    let sink: Arc<dyn ProgressSink> = Arc::new(RecordingSink::new());

    let result = DownloadManager::new()?
        .download(&fixture.url("/x"), &dest, sink)
        .await;

    assert!(matches!(result, Err(DownloadError::Io { .. })));

    Ok(())
}

#[cfg(unix)]
#[tokio::test]
async fn partial_file_stale_with_unremovable_partial_returns_cache_permission_denied() -> Result<()>
{
    use std::os::unix::fs::PermissionsExt;

    let directory = TempDir::new()?;
    let locked_parent = directory.path().join("locked");
    create_dir(&locked_parent).await?;
    let dest = locked_parent.join("model.gguf");
    let partial_path = dest.with_extension("partial");
    write(&partial_path, b"stale").await?;
    let mut perms = metadata(&locked_parent).await?.permissions();
    perms.set_mode(0o500);
    set_permissions(&locked_parent, perms).await?;

    let fixture = LocalHttpFixture::start(Scenario::always(FixtureResponse::status(416))).await?;
    let sink: Arc<dyn ProgressSink> = Arc::new(RecordingSink::new());

    let result = DownloadManager::new()?
        .download(&fixture.url("/x"), &dest, sink)
        .await;

    let mut restore = metadata(&locked_parent).await?.permissions();
    restore.set_mode(0o700);
    set_permissions(&locked_parent, restore).await?;

    assert!(matches!(
        result,
        Err(DownloadError::CachePermissionDenied { .. })
    ));

    Ok(())
}

#[cfg(unix)]
#[tokio::test]
async fn truncate_error_during_ignore_range_returns_io() -> Result<()> {
    let directory = TempDir::new()?;
    let dest = directory.path().join("model.gguf");
    let partial_path = dest.with_extension("partial");
    create_dir(&partial_path).await?;

    let fixture =
        LocalHttpFixture::start(Scenario::always(FixtureResponse::ok(b"body".to_vec()))).await?;
    let sink: Arc<dyn ProgressSink> = Arc::new(RecordingSink::new());

    let result = DownloadManager::new()?
        .download(&fixture.url("/x"), &dest, sink)
        .await;

    assert!(matches!(result, Err(DownloadError::Io { .. })));

    Ok(())
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn download_returns_cache_disk_full_when_target_is_dev_full() -> Result<()> {
    let directory = TempDir::new()?;
    let dest = directory.path().join("model.gguf");
    let partial_path = dest.with_extension("partial");
    std::os::unix::fs::symlink("/dev/full", &partial_path)?;

    let fixture = LocalHttpFixture::start(Scenario::always(FixtureResponse::ok(
        b"this body will fail to write because /dev/full".to_vec(),
    )))
    .await?;
    let sink: Arc<dyn ProgressSink> = Arc::new(RecordingSink::new());

    let result = DownloadManager::new()?
        .download(&fixture.url("/x"), &dest, sink)
        .await;

    let Err(DownloadError::CacheDiskFull { source, .. }) = result else {
        bail!("expected CacheDiskFull, got {result:?}");
    };
    assert_eq!(source.raw_os_error(), Some(28));

    Ok(())
}

#[tokio::test]
async fn download_succeeds_after_cache_dir_was_deleted_between_calls() -> Result<()> {
    let directory = TempDir::new()?;
    let cache_subdir = directory.path().join("cache");
    let dest = cache_subdir.join("model.gguf");

    let body = b"model bytes for the recreation test".to_vec();
    let fixture =
        LocalHttpFixture::start(Scenario::always(FixtureResponse::ok(body.clone()))).await?;
    let url = fixture.url("/x");

    DownloadManager::new()?
        .download(
            &url,
            &dest,
            Arc::new(RecordingSink::new()) as Arc<dyn ProgressSink>,
        )
        .await?;
    assert_eq!(read(&dest).await?, body);

    remove_dir_all(&cache_subdir).await?;

    DownloadManager::new()?
        .download(
            &url,
            &dest,
            Arc::new(RecordingSink::new()) as Arc<dyn ProgressSink>,
        )
        .await?;
    assert_eq!(read(&dest).await?, body);

    Ok(())
}
