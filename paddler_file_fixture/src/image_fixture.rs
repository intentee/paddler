use crate::file_fixture_error::FileFixtureError;
use crate::fixture_base64::fixture_base64;

#[derive(Clone, Copy, Debug)]
pub enum ImageFixture {
    LlamasJpeg,
    LlamasWebp,
    SarnowJpeg,
    SolidColor4000x4000Png,
}

impl ImageFixture {
    /// # Errors
    ///
    /// Returns [`FileFixtureError::FixtureUnreadable`] when the image fixture cannot be read.
    pub fn data_uri(self) -> Result<String, FileFixtureError> {
        Ok(format!(
            "data:{};base64,{}",
            self.mime_type(),
            fixture_base64(self.fixture_name())?
        ))
    }

    const fn fixture_name(self) -> &'static str {
        match self {
            Self::LlamasJpeg => "llamas.jpg",
            Self::LlamasWebp => "llamas.webp",
            Self::SarnowJpeg => "sarnow.jpeg",
            Self::SolidColor4000x4000Png => "solid_color_4000x4000.png",
        }
    }

    const fn mime_type(self) -> &'static str {
        match self {
            Self::LlamasJpeg | Self::SarnowJpeg => "image/jpeg",
            Self::LlamasWebp => "image/webp",
            Self::SolidColor4000x4000Png => "image/png",
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs::read;

    use data_url::DataUrl;

    use super::ImageFixture;
    use crate::fixture_path::fixture_path;

    #[test]
    fn the_llamas_photo_encodes_as_a_jpeg_data_uri() {
        let encoded_fixture = ImageFixture::LlamasJpeg
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
            read(fixture_path(ImageFixture::LlamasJpeg.fixture_name()))
                .expect("the llamas fixture must read")
        );
    }
}
