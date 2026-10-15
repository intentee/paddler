use crate::image_fixture::ImageFixture;

pub const JPEG_AND_WEBP_IMAGE_FIXTURES: [ImageFixture; 2] = [
    ImageFixture {
        fixture_name: "sarnow.jpeg",
        mime_type: "image/jpeg",
    },
    ImageFixture {
        fixture_name: "llamas.webp",
        mime_type: "image/webp",
    },
];
