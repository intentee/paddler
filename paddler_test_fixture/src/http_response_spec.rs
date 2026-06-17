use crate::http_header::HttpHeader;

pub struct HttpResponseSpec {
    pub body: Vec<u8>,
    pub headers: Vec<HttpHeader>,
    pub phantom_content_length_bytes: usize,
    pub status_line: String,
}

impl HttpResponseSpec {
    #[must_use]
    pub fn status(code: u16, reason: &str) -> Self {
        Self {
            body: Vec::new(),
            headers: Vec::new(),
            phantom_content_length_bytes: 0,
            status_line: format!("HTTP/1.1 {code} {reason}"),
        }
    }

    #[must_use]
    pub fn ok_body(body: Vec<u8>) -> Self {
        Self {
            body,
            headers: Vec::new(),
            phantom_content_length_bytes: 0,
            status_line: "HTTP/1.1 200 OK".to_owned(),
        }
    }

    #[must_use]
    pub fn ok_with_headers(headers: Vec<HttpHeader>) -> Self {
        Self {
            body: Vec::new(),
            headers,
            phantom_content_length_bytes: 0,
            status_line: "HTTP/1.1 200 OK".to_owned(),
        }
    }

    #[must_use]
    pub fn truncated_body() -> Self {
        Self {
            body: Vec::new(),
            headers: Vec::new(),
            phantom_content_length_bytes: 1,
            status_line: "HTTP/1.1 200 OK".to_owned(),
        }
    }
}
