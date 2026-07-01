use std::time::Duration;

use anyhow::Result;
use anyhow::anyhow;
use cucumber::gherkin::Step;
use cucumber::when;
use futures::future::join_all;
use tokio::time::sleep;

use crate::paddler_world::PaddlerWorld;

#[when(expr = "multiple requests are sent to {string}")]
pub async fn when_multiple_requests_are_sent_to_path(
    world: &mut PaddlerWorld,
    step: &Step,
    path: String,
) -> Result<()> {
    if let Some(table) = step.table.as_ref() {
        let client = reqwest::Client::new();
        let mut tasks = Vec::new();

        for (row_index, row) in table.rows.iter().enumerate() {
            let request_name = row[0].clone();
            let path_clone = path.clone();
            let client_clone = client.clone();

            let task = tokio::spawn(async move {
                sleep(Duration::from_millis(50 * row_index as u64)).await;

                let response = client_clone
                    .get(format!("http://127.0.0.1:8096{path_clone}"))
                    .header("X-Request-Name", request_name.clone())
                    .send()
                    .await;

                (request_name, response)
            });

            tasks.push(task);
        }

        let results = join_all(tasks).await;
        for result in results {
            match result {
                Ok((request_name, Ok(response))) => {
                    world.responses.insert(request_name, response);
                }
                Ok((request_name, Err(e))) => {
                    return Err(anyhow!("Request {} failed: {}", request_name, e));
                }
                Err(err) => {
                    return Err(anyhow!("Task failed: {}", err));
                }
            }
        }
    } else {
        return Err(anyhow!("Step must contain a table"));
    }

    Ok(())
}
