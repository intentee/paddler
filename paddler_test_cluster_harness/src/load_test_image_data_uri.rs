use anyhow::Result;

use crate::load_fixture_data_uri::load_fixture_data_uri;

pub fn load_test_image_data_uri() -> Result<String> {
    load_fixture_data_uri("llamas.jpg", "image/jpeg")
}

#[cfg(test)]
mod tests {
    use super::load_test_image_data_uri;

    #[test]
    fn encodes_the_fixture_as_a_jpeg_data_uri() {
        let data_uri = load_test_image_data_uri().unwrap();

        assert!(data_uri.starts_with("data:image/jpeg;base64,"));
        assert!(data_uri.len() > "data:image/jpeg;base64,".len());
    }
}
