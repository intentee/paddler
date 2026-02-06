use std::pin::Pin;
use std::task::Context;
use std::task::Poll;

use futures_util::Stream;
use futures_util::stream::unfold;
use reqwest::Response;

use crate::Result;

fn make_stream(response: Response) -> impl Stream<Item = Result<String>> + Send {
    unfold(
        (response, String::new()),
        |(mut response, mut buffer)| async move {
            loop {
                if let Some(line_end) = buffer.find('\n') {
                    let line = buffer[..line_end].to_string();
                    buffer = buffer[line_end + 1..].to_string();

                    let line = line.trim_end_matches('\r');

                    if let Some(data) = line.strip_prefix("data: ") {
                        return Some((Ok(data.to_string()), (response, buffer)));
                    }

                    continue;
                }

                match response.chunk().await {
                    Ok(Some(chunk)) => {
                        let text = String::from_utf8_lossy(&chunk);
                        buffer.push_str(&text);
                    }
                    Ok(None) => return None,
                    Err(err) => return Some((Err(err.into()), (response, buffer))),
                }
            }
        },
    )
}

pub struct StreamSse {
    lines: Pin<Box<dyn Stream<Item = Result<String>> + Send>>,
}

impl StreamSse {
    pub fn from_response(response: Response) -> Self {
        let stream = make_stream(response);

        Self {
            lines: Box::pin(stream),
        }
    }
}

impl Stream for StreamSse {
    type Item = Result<String>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.lines.as_mut().poll_next(cx)
    }
}
