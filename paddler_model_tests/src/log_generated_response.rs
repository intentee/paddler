use paddler_types::generated_token_result::GeneratedTokenResult;

#[expect(clippy::print_stderr, reason = "test diagnostic output")]
pub fn log_generated_response(results: &[GeneratedTokenResult]) {
    let token_count = results
        .iter()
        .filter(|result| matches!(result, GeneratedTokenResult::Token(_)))
        .count();

    let full_response: String = results
        .iter()
        .filter_map(|result| match result {
            GeneratedTokenResult::Token(token) => Some(token.as_str()),
            _ => None,
        })
        .collect();

    eprintln!("Response tokens: {token_count}");
    eprintln!("Full response:\n{full_response}");
}
