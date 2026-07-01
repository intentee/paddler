use dashmap::DashMap;

#[derive(Debug, Default)]
pub struct RequestHeadersToBeSet {
    pub headers: DashMap<String, Vec<(String, String)>>,
}

impl RequestHeadersToBeSet {
    pub fn cleanup(&self) {
        self.headers.clear();
    }

    pub fn insert_header(&self, name: String, header: (String, String)) {
        self.headers.entry(name).or_default().push(header);
    }

    pub fn take_headers_for_request(&self, name: &str) -> Vec<(String, String)> {
        match self.headers.remove(name) {
            Some((_, headers)) => headers,
            None => vec![],
        }
    }
}
