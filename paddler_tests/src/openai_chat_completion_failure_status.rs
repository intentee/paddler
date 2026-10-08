use std::future::Future;

use anyhow::Error;
use anyhow::Result;
use reqwest::Client;
use reqwest::StatusCode;
use serde_json::Value;
use serde_json::json;

use paddler_balancer::compatibility::openai_service::openai_api_path::OpenAIApiPath;
use paddler_openai_response_format_validator::openai_validator::OpenAIValidator;
use paddler_test_cluster_harness::cluster::Cluster;

pub fn openai_chat_completion_failure_status(
    cluster: &Cluster,
) -> impl Future<Output = Result<StatusCode>> + Send + use<> {
    let chat_completions_url = cluster
        .balancer
        .compat_openai_base_url()
        .map_err(Error::from)
        .and_then(|compat_openai_base_url| {
            Ok(compat_openai_base_url.join(OpenAIApiPath::CHAT_COMPLETIONS)?)
        });

    async move {
        let response = Client::new()
            .post(chat_completions_url?)
            .json(&json!({
                "model": "test-model",
                "messages": [{"role": "user", "content": "hi"}]
            }))
            .send()
            .await?;
        let status = response.status();

        OpenAIValidator::new()?.validate_error_response(&response.json::<Value>().await?)?;

        Ok(status)
    }
}
