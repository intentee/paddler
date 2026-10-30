use llama_cpp_bindings::token_piece::TokenPiece;

#[derive(Debug, Default)]
pub struct ToolCallBuffer {
    pub accumulated: String,
    pub holds_visible_text: bool,
}

impl ToolCallBuffer {
    pub fn append(&mut self, piece: &TokenPiece) {
        self.accumulated.push_str(piece.raw());
        self.holds_visible_text |= !piece.visible().is_empty();
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.accumulated.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use llama_cpp_bindings::token_piece::TokenPiece;

    use super::ToolCallBuffer;

    #[test]
    fn appending_keeps_the_raw_text_of_markers_and_of_visible_pieces() {
        let mut buffer = ToolCallBuffer::default();

        buffer.append(&TokenPiece::Marker("<tool_call>".to_owned()));
        buffer.append(&TokenPiece::Visible("{\"name\":\"x\"}".to_owned()));

        assert_eq!(buffer.accumulated, "<tool_call>{\"name\":\"x\"}");
    }
}
