use bytes::BytesMut;

use crate::error::Result;
use crate::stream::decode_stream_line::decode_stream_line;

pub struct LineBuffer {
    bytes: BytesMut,
}

impl LineBuffer {
    pub fn new() -> Self {
        Self {
            bytes: BytesMut::new(),
        }
    }

    pub fn push_chunk(&mut self, chunk: &[u8]) {
        self.bytes.extend_from_slice(chunk);
    }

    pub fn take_line(&mut self) -> Option<Result<String>> {
        let newline_position = self.bytes.iter().position(|byte| *byte == b'\n')?;
        let mut line_bytes = self.bytes.split_to(newline_position + 1);

        line_bytes.truncate(newline_position);

        Some(decode_stream_line(line_bytes))
    }

    pub fn take_remainder(&mut self) -> Option<BytesMut> {
        if self.bytes.is_empty() {
            return None;
        }

        Some(self.bytes.split())
    }
}

#[cfg(test)]
mod tests {
    use super::LineBuffer;
    use crate::error::Error;

    fn take_line(line_buffer: &mut LineBuffer) -> String {
        line_buffer
            .take_line()
            .expect("a complete line must be available")
            .expect("the line must be valid UTF-8")
    }

    #[test]
    fn reassembles_a_multibyte_character_split_across_chunks() {
        let duck = "🦆";
        let duck_bytes = duck.as_bytes();
        let mut line_buffer = LineBuffer::new();

        line_buffer.push_chunk(&duck_bytes[..2]);

        assert!(line_buffer.take_line().is_none());

        line_buffer.push_chunk(&duck_bytes[2..]);
        line_buffer.push_chunk(b"\n");

        assert_eq!(take_line(&mut line_buffer), duck);
    }

    #[test]
    fn errors_on_a_line_that_is_not_valid_utf8() {
        let mut line_buffer = LineBuffer::new();

        line_buffer.push_chunk(&[0xff, 0xfe, b'\n']);

        assert!(matches!(
            line_buffer
                .take_line()
                .expect("a complete line must be available"),
            Err(Error::NonUtf8StreamLine { source }) if source.as_bytes() == [0xff, 0xfe]
        ));
    }

    #[test]
    fn hands_out_the_unterminated_remainder_undecoded() {
        let mut line_buffer = LineBuffer::new();

        line_buffer.push_chunk(&[0xff, 0xfe]);

        assert_eq!(
            line_buffer.take_remainder().as_deref(),
            Some([0xff, 0xfe].as_slice())
        );
    }

    #[test]
    fn continues_a_partial_line_pushed_after_earlier_lines_were_taken() {
        let mut line_buffer = LineBuffer::new();

        line_buffer.push_chunk(b"first\nsecond\nthi");

        assert_eq!(take_line(&mut line_buffer), "first");
        assert_eq!(take_line(&mut line_buffer), "second");
        assert!(line_buffer.take_line().is_none());

        line_buffer.push_chunk(b"rd\n");

        assert_eq!(take_line(&mut line_buffer), "third");
        assert!(line_buffer.take_remainder().is_none());
    }

    #[test]
    fn an_exhausted_buffer_has_no_remainder() {
        let mut line_buffer = LineBuffer::new();

        line_buffer.push_chunk(b"line\n");

        assert_eq!(take_line(&mut line_buffer), "line");
        assert!(line_buffer.take_remainder().is_none());
    }
}
