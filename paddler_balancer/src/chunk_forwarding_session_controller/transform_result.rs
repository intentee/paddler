pub enum TransformResult {
    Chunk(String),
    Discard,
    Error(String),
}

#[cfg(test)]
impl TransformResult {
    #[must_use]
    pub fn chunk_body(&self) -> Option<&str> {
        match self {
            Self::Chunk(body) => Some(body),
            Self::Discard | Self::Error(_) => None,
        }
    }

    #[must_use]
    pub fn error_body(&self) -> Option<&str> {
        match self {
            Self::Error(body) => Some(body),
            Self::Chunk(_) | Self::Discard => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::TransformResult;

    #[test]
    fn chunk_body_returns_inner_string_only_for_chunk_variant() {
        assert_eq!(
            TransformResult::Chunk("hello".to_owned()).chunk_body(),
            Some("hello")
        );
        assert_eq!(TransformResult::Discard.chunk_body(), None);
        assert_eq!(TransformResult::Error("boom".to_owned()).chunk_body(), None);
    }

    #[test]
    fn error_body_returns_inner_string_only_for_error_variant() {
        assert_eq!(
            TransformResult::Error("boom".to_owned()).error_body(),
            Some("boom")
        );
        assert_eq!(TransformResult::Discard.error_body(), None);
        assert_eq!(
            TransformResult::Chunk("hello".to_owned()).error_body(),
            None
        );
    }
}
