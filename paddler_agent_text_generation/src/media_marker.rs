use rand::RngExt as _;
use rand::rng;

#[derive(Debug)]
pub struct MediaMarker {
    pub marker: String,
}

impl MediaMarker {
    #[must_use]
    pub const fn new(marker: String) -> Self {
        Self { marker }
    }

    #[must_use]
    pub fn unpredictable() -> Self {
        Self::new(format!("<__media_{:032x}__>", rng().random::<u128>()))
    }
}
