use std::mem::take;

#[derive(Debug, Default)]
pub struct ToolCallBuffer {
    accumulated: String,
}

impl ToolCallBuffer {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            accumulated: String::new(),
        }
    }

    pub fn append(&mut self, fragment: &str) {
        self.accumulated.push_str(fragment);
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.accumulated
    }

    pub fn clear(&mut self) {
        self.accumulated.clear();
    }

    #[must_use]
    pub fn take(&mut self) -> String {
        take(&mut self.accumulated)
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.accumulated.is_empty()
    }

    #[must_use]
    pub const fn len(&self) -> usize {
        self.accumulated.len()
    }
}

#[cfg(test)]
mod tests {
    use super::ToolCallBuffer;

    #[test]
    fn new_is_empty() {
        let buffer = ToolCallBuffer::new();

        assert!(buffer.is_empty());
        assert_eq!(buffer.len(), 0);
        assert_eq!(buffer.as_str(), "");
    }

    #[test]
    fn append_extends_buffer() {
        let mut buffer = ToolCallBuffer::new();
        buffer.append("hello");

        assert_eq!(buffer.as_str(), "hello");
        assert_eq!(buffer.len(), 5);
        assert!(!buffer.is_empty());
    }

    #[test]
    fn multiple_appends_concatenate() {
        let mut buffer = ToolCallBuffer::new();
        buffer.append("<tool_call>\n");
        buffer.append("{\"name\":\"x\"}");
        buffer.append("\n</tool_call>");

        assert_eq!(
            buffer.as_str(),
            "<tool_call>\n{\"name\":\"x\"}\n</tool_call>"
        );
    }

    #[test]
    fn clear_resets_to_empty() {
        let mut buffer = ToolCallBuffer::new();
        buffer.append("data");
        buffer.clear();

        assert!(buffer.is_empty());
        assert_eq!(buffer.len(), 0);
    }

    #[test]
    fn take_returns_contents_and_clears() {
        let mut buffer = ToolCallBuffer::new();
        buffer.append("hello");
        let taken = buffer.take();

        assert_eq!(taken, "hello");
        assert!(buffer.is_empty());
    }

    #[test]
    fn take_on_empty_returns_empty_string() {
        let mut buffer = ToolCallBuffer::new();
        let taken = buffer.take();

        assert!(taken.is_empty());
        assert!(buffer.is_empty());
    }

    #[test]
    fn append_handles_unicode() {
        let mut buffer = ToolCallBuffer::new();
        buffer.append("héllo");

        assert_eq!(buffer.as_str(), "héllo");
        assert_eq!(buffer.len(), "héllo".len());
    }
}
