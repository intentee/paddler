use std::marker::PhantomData;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;

use futures_util::Stream;
use futures_util::stream::unfold;
use reqwest::Response;
use serde::de::DeserializeOwned;
use serde_json::from_str;

use crate::Result;

fn make_stream<TItem: DeserializeOwned + Send + 'static>(
    response: Response,
) -> impl Stream<Item = Result<TItem>> + Send {
    unfold(
        (response, String::new(), PhantomData::<TItem>),
        |(mut response, mut buffer, _item_type_marker)| async move {
            loop {
                if let Some(line_end) = buffer.find('\n') {
                    let line = buffer[..line_end].trim().to_string();
                    buffer = buffer[line_end + 1..].to_string();

                    if line.is_empty() {
                        continue;
                    }

                    let result: Result<TItem> = from_str(&line).map_err(Into::into);

                    return Some((result, (response, buffer, PhantomData)));
                }

                match response.chunk().await {
                    Ok(Some(chunk)) => {
                        let text = String::from_utf8_lossy(&chunk);
                        buffer.push_str(&text);
                    }
                    Ok(None) => {
                        let remaining = buffer.trim().to_string();
                        if !remaining.is_empty() {
                            buffer.clear();
                            let result: Result<TItem> = from_str(&remaining).map_err(Into::into);

                            return Some((result, (response, buffer, PhantomData)));
                        }

                        return None;
                    }
                    Err(err) => {
                        return Some((Err(err.into()), (response, buffer, PhantomData)));
                    }
                }
            }
        },
    )
}

pub struct StreamNdjson<TItem> {
    inner: Pin<Box<dyn Stream<Item = Result<TItem>> + Send>>,
}

impl<TItem: DeserializeOwned + Send + 'static> StreamNdjson<TItem> {
    pub fn from_response(response: Response) -> Self {
        let stream = make_stream::<TItem>(response);

        Self {
            inner: Box::pin(stream),
        }
    }
}

impl<TItem> Stream for StreamNdjson<TItem> {
    type Item = Result<TItem>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.inner.as_mut().poll_next(cx)
    }
}
