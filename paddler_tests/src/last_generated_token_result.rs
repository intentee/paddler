use tokio::sync::mpsc::UnboundedReceiver;

use paddler_messaging::generated_token_result::GeneratedTokenResult;

#[must_use]
pub fn last_generated_token_result(
    mut generated_tokens_rx: UnboundedReceiver<GeneratedTokenResult>,
) -> Option<GeneratedTokenResult> {
    let mut last_result = None;

    while let Ok(generated_token_result) = generated_tokens_rx.try_recv() {
        last_result = Some(generated_token_result);
    }

    last_result
}
