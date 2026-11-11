pub struct MediaMarker {
    pub marker: String,
}

impl MediaMarker {
    #[must_use]
    pub const fn new(marker: String) -> Self {
        Self { marker }
    }
}
