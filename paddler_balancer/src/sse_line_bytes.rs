use bytes::Bytes;

use crate::chunk_forwarding_session_controller::transform_result::TransformResult;

#[must_use]
pub fn sse_line_bytes(result: TransformResult) -> Option<Bytes> {
    match result {
        TransformResult::Chunk(content) | TransformResult::Error(content) => {
            Some(Bytes::from(format!("{content}\n")))
        }
        TransformResult::Discard => None,
    }
}

#[cfg(test)]
mod tests {
    use super::sse_line_bytes;
    use crate::chunk_forwarding_session_controller::transform_result::TransformResult;

    #[test]
    fn a_chunk_becomes_a_newline_terminated_line() {
        let bytes = sse_line_bytes(TransformResult::Chunk("hello".to_owned()));

        assert_eq!(bytes.as_deref(), Some(&b"hello\n"[..]));
    }

    #[test]
    fn an_error_becomes_a_newline_terminated_line() {
        let bytes = sse_line_bytes(TransformResult::Error("boom".to_owned()));

        assert_eq!(bytes.as_deref(), Some(&b"boom\n"[..]));
    }

    #[test]
    fn a_discard_produces_no_line() {
        let bytes = sse_line_bytes(TransformResult::Discard);

        assert_eq!(bytes, None);
    }
}
