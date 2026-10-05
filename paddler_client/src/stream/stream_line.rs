use bytes::BytesMut;

pub enum StreamLine {
    Terminated(String),
    Unterminated(BytesMut),
}
