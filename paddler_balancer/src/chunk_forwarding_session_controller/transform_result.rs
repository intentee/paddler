use bytes::Bytes;

pub enum TransformResult {
    Chunk(String),
    Discard,
    Error(String),
}

impl TransformResult {
    #[must_use]
    pub fn into_ndjson_line(self) -> Option<Bytes> {
        match self {
            Self::Chunk(mut line) | Self::Error(mut line) => {
                line.push('\n');

                Some(Bytes::from(line))
            }
            Self::Discard => None,
        }
    }
}
