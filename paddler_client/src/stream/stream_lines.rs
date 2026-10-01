use futures_util::Stream;
use futures_util::stream::unfold;
use reqwest::Response;
use tokio_util::sync::CancellationToken;

use crate::error::Result;
use crate::stream::line_buffer::LineBuffer;
use crate::stream::stream_line::StreamLine;

struct StreamLinesState {
    cancellation_token: CancellationToken,
    is_terminated: bool,
    line_buffer: LineBuffer,
    response: Response,
}

pub fn stream_lines(
    cancellation_token: CancellationToken,
    response: Response,
) -> impl Stream<Item = Result<StreamLine>> + Send {
    unfold(
        StreamLinesState {
            cancellation_token,
            is_terminated: false,
            line_buffer: LineBuffer::new(),
            response,
        },
        |mut state| async move {
            if state.is_terminated || state.cancellation_token.is_cancelled() {
                return None;
            }

            loop {
                if let Some(line_result) = state.line_buffer.take_line() {
                    return Some((line_result.map(StreamLine::Terminated), state));
                }

                let chunk_result = state
                    .cancellation_token
                    .run_until_cancelled(state.response.chunk())
                    .await?;

                match chunk_result {
                    Ok(Some(chunk)) => state.line_buffer.push_chunk(&chunk),
                    Ok(None) => {
                        state.is_terminated = true;

                        let remainder = state.line_buffer.take_remainder()?;

                        return Some((Ok(StreamLine::Unterminated(remainder)), state));
                    }
                    Err(transport_error) => {
                        state.is_terminated = true;

                        return Some((Err(transport_error.into()), state));
                    }
                }
            }
        },
    )
}
