use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result;

pub struct MediaMarker {
    pub marker: String,
}

impl MediaMarker {
    pub fn new(marker: String) -> Self {
        Self { marker }
    }
}

impl Display for MediaMarker {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> Result {
        write!(formatter, "{}", self.marker)
    }
}
