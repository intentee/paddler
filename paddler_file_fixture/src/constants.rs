use crate::image_fixture::ImageFixture;

pub const CA_BUNDLE_WITHOUT_CERTIFICATES: &str = "ca_bundle_without_certificates.pem";
pub const LLAMAS_JPEG: ImageFixture = ImageFixture {
    fixture_name: "llamas.jpg",
    mime_type: "image/jpeg",
};
pub const LLAMAS_WEBP: ImageFixture = ImageFixture {
    fixture_name: "llamas.webp",
    mime_type: "image/webp",
};
pub const SARNOW_JPEG: ImageFixture = ImageFixture {
    fixture_name: "sarnow.jpeg",
    mime_type: "image/jpeg",
};
pub const SOLID_COLOR_4000X4000_PNG: ImageFixture = ImageFixture {
    fixture_name: "solid_color_4000x4000.png",
    mime_type: "image/png",
};

#[cfg(test)]
mod tests {
    use std::fs::read;

    use data_url::DataUrl;

    use super::LLAMAS_JPEG;
    use crate::fixture_path::fixture_path;

    #[test]
    fn the_llamas_photo_encodes_as_a_jpeg_data_uri() {
        let encoded_fixture = LLAMAS_JPEG
            .data_uri()
            .expect("the llamas fixture must load");
        let parsed_fixture =
            DataUrl::process(&encoded_fixture).expect("the fixture must encode as a data URI");
        let (decoded_body, _fragment) = parsed_fixture
            .decode_to_vec()
            .expect("the data URI must carry base64 data");

        assert_eq!(parsed_fixture.mime_type().type_, "image");
        assert_eq!(parsed_fixture.mime_type().subtype, "jpeg");
        assert_eq!(
            decoded_body,
            read(fixture_path(LLAMAS_JPEG.fixture_name)).expect("the llamas fixture must read")
        );
    }
}
