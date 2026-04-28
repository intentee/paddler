use anyhow::Result;
use paddler_tests::load_test_image_data_uri::load_test_image_data_uri;

#[test]
fn harness_load_test_image_data_uri_returns_jpeg_base64() -> Result<()> {
    let data_uri = load_test_image_data_uri()?;

    assert!(
        data_uri.starts_with("data:image/jpeg;base64,"),
        "expected JPEG data URI prefix; got {data_uri:?}"
    );
    assert!(
        data_uri.len() > "data:image/jpeg;base64,".len(),
        "expected non-empty base64 payload"
    );

    Ok(())
}
