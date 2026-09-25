use llama_cpp_bindings::mtmd::MtmdBitmap;
use llama_cpp_bindings::mtmd::MtmdBitmapError;
use paddler_image_decoder::decoded_image::DecodedImage;

pub trait ConvertsToMtmdBitmap {
    fn to_mtmd_bitmap(&self) -> Result<MtmdBitmap, MtmdBitmapError>;
}

impl ConvertsToMtmdBitmap for DecodedImage {
    fn to_mtmd_bitmap(&self) -> Result<MtmdBitmap, MtmdBitmapError> {
        MtmdBitmap::from_image_data(self.width, self.height, &self.rgb_pixels)
    }
}

#[cfg(test)]
mod tests {
    use paddler_image_decoder::decoded_image::DecodedImage;

    use super::ConvertsToMtmdBitmap;

    #[test]
    fn converts_decoded_pixels_into_bitmap_with_matching_dimensions() {
        let decoded_image = DecodedImage {
            height: 4,
            rgb_pixels: vec![0; 8 * 4 * 3],
            width: 8,
        };

        let bitmap = decoded_image.to_mtmd_bitmap().unwrap();

        assert_eq!(bitmap.nx(), 8);
        assert_eq!(bitmap.ny(), 4);
    }
}
