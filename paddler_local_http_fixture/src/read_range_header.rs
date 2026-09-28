use std::io;
use std::io::ErrorKind;

use http::header::RANGE;
use tokio::io::AsyncReadExt as _;
use tokio::net::TcpStream;

const MAX_REQUEST_HEADERS: usize = 64;

pub async fn read_range_header(stream: &mut TcpStream) -> io::Result<Option<Vec<u8>>> {
    let mut received = Vec::new();

    loop {
        let mut parsed_headers = [httparse::EMPTY_HEADER; MAX_REQUEST_HEADERS];
        let mut request = httparse::Request::new(&mut parsed_headers);

        if request
            .parse(&received)
            .map_err(|source| io::Error::new(ErrorKind::InvalidData, source))?
            .is_complete()
        {
            return Ok(request
                .headers
                .iter()
                .find(|parsed_header| parsed_header.name.eq_ignore_ascii_case(RANGE.as_str()))
                .map(|range_header| range_header.value.to_vec()));
        }

        if stream.read_buf(&mut received).await? == 0 {
            return Err(io::Error::from(ErrorKind::UnexpectedEof));
        }
    }
}
