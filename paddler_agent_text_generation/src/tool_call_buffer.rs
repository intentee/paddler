use std::mem::take;

#[derive(Debug, Default)]
pub struct ToolCallBuffer {
    accumulated: String,
}

impl ToolCallBuffer {
    pub fn append(&mut self, fragment: &str) {
        self.accumulated.push_str(fragment);
    }

    #[must_use]
    pub fn take(&mut self) -> String {
        take(&mut self.accumulated)
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.accumulated.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::ToolCallBuffer;

    #[test]
    fn take_returns_every_appended_fragment_and_empties_the_buffer() {
        let mut buffer = ToolCallBuffer::default();

        buffer.append("<tool_call>\n");
        buffer.append("{\"name\":\"x\"}");

        assert_eq!(buffer.take(), "<tool_call>\n{\"name\":\"x\"}");
        assert!(buffer.is_empty());
    }
}
