use anyhow::Result;
use tempfile::NamedTempFile;
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use tokio::process::Child;

pub struct AcceptedRequestResult {
    pub accepted: bool,
    pub contents: String,
}

#[derive(Debug)]
pub struct LlamaCppInstance {
    pub child: Child,
    pub name: String,
    pub log_file: NamedTempFile,
    pub port: u16,
}

impl LlamaCppInstance {
    /// Requests are stored in a log file in CSV format `instance_name;request_name`
    pub async fn accepted_request(&self, request_name: &str) -> Result<AcceptedRequestResult> {
        let mut file = File::open(&self.log_file).await?;
        let mut contents = String::new();

        file.read_to_string(&mut contents).await?;

        for line in contents.lines() {
            let parts: Vec<&str> = line.split(';').collect();
            if parts.len() == 2 && parts[0] == self.name && parts[1] == request_name {
                return Ok(AcceptedRequestResult {
                    accepted: true,
                    contents,
                });
            }
        }

        Ok(AcceptedRequestResult {
            accepted: false,
            contents,
        })
    }

    pub async fn cleanup(&mut self) {
        if let Err(err) = self.child.kill().await {
            panic!("Failed to kill llama {}: {}", self.name, err);
        }
    }
}
