use anyhow::Result;
use paddler_types::generated_token_result::GeneratedTokenResult;
use tokio::sync::mpsc;

pub async fn collect_generated_tokens(
    mut generated_tokens_rx: mpsc::UnboundedReceiver<GeneratedTokenResult>,
) -> Result<Vec<GeneratedTokenResult>> {
    let mut results = Vec::new();

    while let Some(generated_token) = generated_tokens_rx.recv().await {
        let is_done = matches!(generated_token, GeneratedTokenResult::Done);

        results.push(generated_token);

        if is_done {
            break;
        }
    }

    Ok(results)
}
