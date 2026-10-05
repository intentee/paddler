use bytes::BytesMut;

use crate::error::Error;
use crate::error::Result;

pub fn decode_stream_line(line_bytes: BytesMut) -> Result<String> {
    String::from_utf8(line_bytes.into()).map_err(|source| Error::NonUtf8StreamLine { source })
}
