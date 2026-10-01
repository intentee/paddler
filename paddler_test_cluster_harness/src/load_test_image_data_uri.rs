use anyhow::Result;

use crate::load_fixture_data_uri::load_fixture_data_uri;

pub fn load_test_image_data_uri() -> Result<String> {
    load_fixture_data_uri("llamas.jpg", "image/jpeg")
}

#[cfg(test)]
mod tests {
    use std::fs::read;

    use data_url::DataUrl;

    use super::load_test_image_data_uri;

    #[test]
    fn encodes_the_fixture_as_a_jpeg_data_uri() {
        let encoded_fixture = load_test_image_data_uri().unwrap();
        let parsed_fixture = DataUrl::process(&encoded_fixture).unwrap();
        let (body, _fragment) = parsed_fixture.decode_to_vec().unwrap();

        assert_eq!(parsed_fixture.mime_type().type_, "image");
        assert_eq!(parsed_fixture.mime_type().subtype, "jpeg");
        assert_eq!(
            body,
            read(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../fixtures/llamas.jpg"
            ))
            .unwrap()
        );
    }
}
