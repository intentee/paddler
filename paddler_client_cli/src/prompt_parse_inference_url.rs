use url::Url;

pub fn prompt_parse_inference_url(input_addr: &str) -> Result<Url, String> {
    Url::parse(&format!("http://{input_addr}"))
        .map_err(|err| format!("invalid address '{input_addr}': {err}"))
}
