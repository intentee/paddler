use anyhow::Context as _;
use anyhow::Result;
use anyhow::bail;
use tokio::io::AsyncBufReadExt as _;
use tokio::io::BufReader;

pub async fn wait_for_log_line<TReader>(
    reader: &mut BufReader<TReader>,
    needle: &str,
    captured_lines: &mut Vec<String>,
) -> Result<()>
where
    TReader: tokio::io::AsyncRead + Unpin,
{
    let mut line = String::new();

    loop {
        line.clear();

        let bytes_read = reader
            .read_line(&mut line)
            .await
            .context("failed to read paddler_gui output")?;

        if bytes_read == 0 {
            bail!(
                "paddler_gui output ended before emitting {needle:?}; captured output:\n{}",
                captured_lines.join("")
            );
        }

        captured_lines.push(line.clone());

        if line.contains(needle) {
            return Ok(());
        }
    }
}
