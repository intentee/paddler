use paddler_types::generated_token_result::GeneratedTokenResult;

pub fn log_generated_response(results: &[GeneratedTokenResult]) {
    let thinking_token_count = results
        .iter()
        .filter(|result| matches!(result, GeneratedTokenResult::ThinkingToken(_)))
        .count();

    let response_token_count = results
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

    if thinking_token_count > 0 {
        eprintln!(
            "Thinking tokens: {thinking_token_count}, Response tokens: {response_token_count}"
        );
    } else {
        eprintln!("Response tokens: {response_token_count}");
    }

    eprintln!("Full response:\n{full_response}");
}
