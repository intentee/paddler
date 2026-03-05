use anyhow::Result;
use anyhow::anyhow;
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use paddler_types::image_url::ImageUrl;

#[derive(Debug)]
pub struct DecodedImage {
    pub data: Vec<u8>,
}

impl DecodedImage {
    pub fn from_data_uri(image_url: &ImageUrl) -> Result<Self> {
        let url = &image_url.url;

        let after_data = url.strip_prefix("data:").ok_or_else(|| {
            anyhow!(
                "Remote image URLs are not supported. Use base64 data URIs (data:image/...;base64,...) instead."
            )
        })?;

        let (_metadata, encoded_data) = after_data
            .split_once(',')
            .ok_or_else(|| anyhow!("Invalid data URI: missing comma separator"))?;

        let data = BASE64_STANDARD.decode(encoded_data)?;

        Ok(Self { data })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decodes_valid_png_data_uri() {
        let png_bytes: Vec<u8> = vec![0x89, 0x50, 0x4E, 0x47];
        let encoded = BASE64_STANDARD.encode(&png_bytes);
        let image_url = ImageUrl {
            url: format!("data:image/png;base64,{encoded}"),
        };

        let result = DecodedImage::from_data_uri(&image_url).unwrap();

        assert_eq!(result.data, png_bytes);
    }

    #[test]
    fn test_rejects_remote_url() {
        let image_url = ImageUrl {
            url: "https://example.com/image.png".to_string(),
        };

        let result = DecodedImage::from_data_uri(&image_url);

        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Remote image URLs are not supported")
        );
    }

    #[test]
    fn test_rejects_data_uri_without_comma() {
        let image_url = ImageUrl {
            url: "data:image/png;base64".to_string(),
        };

        let result = DecodedImage::from_data_uri(&image_url);

        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("missing comma separator")
        );
    }

    #[test]
    fn test_rejects_invalid_base64_payload() {
        let image_url = ImageUrl {
            url: "data:image/png;base64,!!!not-valid-base64!!!".to_string(),
        };

        let result = DecodedImage::from_data_uri(&image_url);

        assert!(result.is_err());
    }
}
