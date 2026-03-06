use paddler_types::generated_token_result::GeneratedTokenResult;

const THINK_OPEN_TAG: &str = "<think>";
const THINK_CLOSE_TAG: &str = "</think>";
const THINK_CLOSE_TAG_MAX_PARTIAL_LEN: usize = THINK_CLOSE_TAG.len() - 1;

pub struct GeneratedTokenPostProcessor {
    buffer: String,
    is_first_token: bool,
    is_thinking: bool,
}

impl GeneratedTokenPostProcessor {
    pub fn new(enable_thinking: bool) -> Self {
        Self {
            buffer: String::new(),
            is_first_token: true,
            is_thinking: enable_thinking,
        }
    }

    pub fn flush(&mut self) -> Vec<GeneratedTokenResult> {
        if self.buffer.is_empty() {
            return vec![];
        }

        let content = self.drain_buffer();

        vec![self.wrap_token(content)]
    }

    pub fn push(&mut self, token: &str) -> Vec<GeneratedTokenResult> {
        if token.is_empty() {
            return vec![];
        }

        if !self.is_thinking {
            return vec![GeneratedTokenResult::Token(token.to_string())];
        }

        self.buffer.push_str(token);
        self.strip_think_open_tag_prefix();

        if self.buffer.is_empty() {
            return vec![];
        }

        self.push_thinking_token()
    }

    #[inline]
    fn drain_buffer(&mut self) -> String {
        std::mem::take(&mut self.buffer)
    }

    fn drain_buffer_from(&mut self, byte_offset: usize) -> String {
        let tail = self.buffer[byte_offset..].to_string();

        self.buffer.truncate(byte_offset);

        tail
    }

    fn drain_buffer_up_to(&mut self, byte_offset: usize) -> String {
        let head = self.buffer[..byte_offset].to_string();

        self.buffer = self.buffer[byte_offset..].to_string();

        head
    }

    fn strip_think_open_tag_prefix(&mut self) {
        if !self.is_first_token {
            return;
        }

        if self.buffer.starts_with(THINK_OPEN_TAG) {
            self.is_first_token = false;
            self.buffer = self.buffer[THINK_OPEN_TAG.len()..].to_string();
        } else if !THINK_OPEN_TAG.starts_with(&self.buffer) {
            self.is_first_token = false;
        }
    }

    fn wrap_token(&self, content: String) -> GeneratedTokenResult {
        if self.is_thinking {
            GeneratedTokenResult::ThinkingToken(content)
        } else {
            GeneratedTokenResult::Token(content)
        }
    }

    fn push_thinking_token(&mut self) -> Vec<GeneratedTokenResult> {
        if let Some(tag_position) = self.buffer.find(THINK_CLOSE_TAG) {
            let after_thinking = self.drain_buffer_from(tag_position);
            let thinking_content = self.drain_buffer();
            let response_content = after_thinking[THINK_CLOSE_TAG.len()..].to_string();

            self.is_thinking = false;

            let mut results = Vec::new();

            if !thinking_content.is_empty() {
                results.push(GeneratedTokenResult::ThinkingToken(thinking_content));
            }

            if !response_content.is_empty() {
                results.push(GeneratedTokenResult::Token(response_content));
            }

            return results;
        }

        let target_flush_len = self
            .buffer
            .len()
            .saturating_sub(THINK_CLOSE_TAG_MAX_PARTIAL_LEN);

        if target_flush_len == 0 {
            return vec![];
        }

        let safe_flush_len = self.buffer.floor_char_boundary(target_flush_len);

        if safe_flush_len == 0 {
            return vec![];
        }

        let to_send = self.drain_buffer_up_to(safe_flush_len);

        vec![GeneratedTokenResult::ThinkingToken(to_send)]
    }
}

#[cfg(test)]
mod tests {
    use paddler_types::generated_token_result::GeneratedTokenResult;

    use super::GeneratedTokenPostProcessor;

    fn all_are_thinking_tokens(results: &[GeneratedTokenResult]) -> bool {
        results
            .iter()
            .all(|result| matches!(result, GeneratedTokenResult::ThinkingToken(_)))
    }

    fn collect_thinking_text(results: &[GeneratedTokenResult]) -> String {
        results
            .iter()
            .filter_map(|result| match result {
                GeneratedTokenResult::ThinkingToken(text) => Some(text.as_str()),
                _ => None,
            })
            .collect()
    }

    fn contains_thinking_token(results: &[GeneratedTokenResult], expected: &str) -> bool {
        results.iter().any(
            |result| matches!(result, GeneratedTokenResult::ThinkingToken(text) if text == expected),
        )
    }

    fn contains_token(results: &[GeneratedTokenResult], expected: &str) -> bool {
        results
            .iter()
            .any(|result| matches!(result, GeneratedTokenResult::Token(text) if text == expected))
    }

    #[test]
    fn thinking_disabled_passes_tokens_through() {
        let mut processor = GeneratedTokenPostProcessor::new(false);

        let results = processor.push("hello");

        assert_eq!(results.len(), 1);
        assert!(contains_token(&results, "hello"));
    }

    #[test]
    fn thinking_disabled_all_tokens_are_regular() {
        let mut processor = GeneratedTokenPostProcessor::new(false);

        let results_first = processor.push("first");
        let results_second = processor.push("second");

        assert!(contains_token(&results_first, "first"));
        assert!(contains_token(&results_second, "second"));
    }

    #[test]
    fn think_ignore_doubled_thinking_token() {
        let mut processor = GeneratedTokenPostProcessor::new(true);

        let results = processor.push("<think>I am thinking</think>response");

        assert_eq!(results.len(), 2);
        assert!(contains_thinking_token(&results, "I am thinking"));
        assert!(contains_token(&results, "response"));
    }

    #[test]
    fn think_close_tag_in_single_token_splits_correctly() {
        let mut processor = GeneratedTokenPostProcessor::new(true);

        let results = processor.push("I am thinking</think>response");

        assert_eq!(results.len(), 2);
        assert!(contains_thinking_token(&results, "I am thinking"));
        assert!(contains_token(&results, "response"));
    }

    #[test]
    fn think_close_tag_split_across_two_tokens() {
        let mut processor = GeneratedTokenPostProcessor::new(true);

        let results_first = processor.push("thinking content</thi");

        assert!(!results_first.is_empty());
        assert!(all_are_thinking_tokens(&results_first));

        let results_second = processor.push("nk>response text");

        assert!(contains_token(&results_second, "response text"));
    }

    #[test]
    fn content_after_think_close_tag_emitted_as_token() {
        let mut processor = GeneratedTokenPostProcessor::new(true);

        let results = processor.push("</think>hello world");

        assert_eq!(results.len(), 1);
        assert!(contains_token(&results, "hello world"));
    }

    #[test]
    fn no_think_close_tag_flushes_as_thinking_token() {
        let mut processor = GeneratedTokenPostProcessor::new(true);

        let results_push = processor.push("all thinking no closing");

        assert!(!results_push.is_empty());
        assert!(all_are_thinking_tokens(&results_push));

        let results_flush = processor.flush();

        assert!(results_flush.is_empty() || all_are_thinking_tokens(&results_flush));

        let all_content =
            collect_thinking_text(&results_push) + &collect_thinking_text(&results_flush);

        assert_eq!(all_content, "all thinking no closing");
    }

    #[test]
    fn flush_remaining_thinking_buffer() {
        let mut processor = GeneratedTokenPostProcessor::new(true);

        processor.push("partial</thi");

        let results = processor.flush();

        assert!(!results.is_empty());
        assert!(all_are_thinking_tokens(&results));
    }

    #[test]
    fn tokens_after_switching_to_response_mode_are_regular_tokens() {
        let mut processor = GeneratedTokenPostProcessor::new(true);

        processor.push("</think>");

        let results = processor.push("response token");

        assert_eq!(results.len(), 1);
        assert!(contains_token(&results, "response token"));
    }

    #[test]
    fn flush_when_buffer_is_empty_returns_nothing() {
        let mut processor = GeneratedTokenPostProcessor::new(true);

        let results = processor.flush();

        assert!(results.is_empty());
    }

    #[test]
    fn multibyte_characters_in_thinking_buffer_do_not_panic() {
        let mut processor = GeneratedTokenPostProcessor::new(true);

        let results = processor.push(" \"😊\" or");

        assert!(all_are_thinking_tokens(&results));
    }

    #[test]
    fn think_open_tag_in_middle_of_buffer_is_not_stripped() {
        let mut processor = GeneratedTokenPostProcessor::new(true);

        let results_first = processor.push("some content");
        let results_second = processor.push("<think>more content</think>response");

        let all_thinking =
            collect_thinking_text(&results_first) + &collect_thinking_text(&results_second);

        assert!(
            all_thinking.contains("<think>"),
            "Expected <think> to be preserved in the middle of the buffer, got: {all_thinking}"
        );

        assert!(contains_token(&results_second, "response"));
    }

    #[test]
    fn think_open_tag_split_across_first_tokens_is_stripped() {
        let mut processor = GeneratedTokenPostProcessor::new(true);

        let results_first = processor.push("<thi");

        assert!(results_first.is_empty());

        let results_second = processor.push("nk>I am thinking</think>response");

        assert!(contains_thinking_token(&results_second, "I am thinking"));
        assert!(contains_token(&results_second, "response"));
    }

    #[test]
    fn empty_token_returns_nothing() {
        let mut processor = GeneratedTokenPostProcessor::new(false);

        let results = processor.push("");

        assert!(results.is_empty());
    }
}
