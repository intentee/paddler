use std::io;
use std::net::Ipv4Addr;
use std::net::SocketAddr;

use futures_util::TryFutureExt as _;
use socket2::SockRef;
use tokio::io::AsyncWriteExt as _;
use tokio::net::TcpSocket;
use tokio::net::TcpStream;

use paddler_tcp_buffer_limits::tcp_buffer_limits_file::TcpBufferLimitsFile;

use crate::local_http_fixture::LocalHttpFixture;
use crate::resetting_client_error::ResettingClientError;

const BYTES_BEYOND_BOTH_SOCKET_BUFFERS: usize = 1;
const REQUEST: &[u8] = b"GET / HTTP/1.1\r\nHost: fixture\r\n\r\n";
const SMALLEST_RECEIVE_BUFFER_REQUEST: usize = 1;

async fn send_request(mut client: TcpStream) -> io::Result<TcpStream> {
    client.write_all(REQUEST).await.map(|()| client)
}

pub struct ResettingClient {
    receive_buffer: usize,
    socket: TcpSocket,
}

impl ResettingClient {
    /// # Errors
    ///
    /// Returns [`ResettingClientError::SocketUnopenable`] when the client socket cannot be opened, [`ResettingClientError::ReceiveBufferUnshrinkable`] when its receive buffer cannot be shrunk, and [`ResettingClientError::ReceiveBufferUnreadable`] when its receive buffer size cannot be read back.
    pub fn with_the_smallest_receive_buffer() -> Result<Self, ResettingClientError> {
        TcpSocket::new_v4()
            .map_err(ResettingClientError::SocketUnopenable)
            .and_then(|socket| {
                SockRef::from(&socket)
                    .set_recv_buffer_size(SMALLEST_RECEIVE_BUFFER_REQUEST)
                    .map_err(ResettingClientError::ReceiveBufferUnshrinkable)
                    .and_then(|()| {
                        SockRef::from(&socket)
                            .recv_buffer_size()
                            .map_err(ResettingClientError::ReceiveBufferUnreadable)
                    })
                    .map(|receive_buffer| Self {
                        receive_buffer,
                        socket,
                    })
            })
    }

    /// # Errors
    ///
    /// Returns [`ResettingClientError::SendBufferLimitsUnreadable`] when the TCP send buffer limits cannot be read, and [`ResettingClientError::OutlastingBodyLengthOverflows`] when no body length outlasts both the largest send buffer and the receive buffer.
    pub fn body_outlasting_both_socket_buffers(
        &self,
        send_buffer_limits: &TcpBufferLimitsFile,
    ) -> Result<Vec<u8>, ResettingClientError> {
        send_buffer_limits
            .largest_buffer()
            .map_err(ResettingClientError::SendBufferLimitsUnreadable)
            .and_then(|largest_send_buffer| {
                largest_send_buffer
                    .checked_add(self.receive_buffer)
                    .and_then(|both_socket_buffers| {
                        both_socket_buffers.checked_add(BYTES_BEYOND_BOTH_SOCKET_BUFFERS)
                    })
                    .ok_or(ResettingClientError::OutlastingBodyLengthOverflows {
                        largest_send_buffer,
                        receive_buffer: self.receive_buffer,
                    })
            })
            .map(|outlasting_body_length| vec![0; outlasting_body_length])
    }

    /// # Errors
    ///
    /// Returns [`ResettingClientError::FixtureUnreachable`] when the client cannot connect to the fixture, [`ResettingClientError::RequestUnsent`] when it cannot send its request, [`ResettingClientError::RequestUnobserved`] when the fixture stops counting requests before it reads that request, and [`ResettingClientError::ZeroLingerUnset`] when the client cannot make closing its connection reset it.
    pub async fn request_then_reset(
        self,
        fixture: &LocalHttpFixture,
    ) -> Result<(), ResettingClientError> {
        let connected_client: Result<TcpStream, ResettingClientError> = self
            .socket
            .connect(SocketAddr::from((Ipv4Addr::LOCALHOST, fixture.addr.port())))
            .map_err(ResettingClientError::FixtureUnreachable)
            .and_then(|client| send_request(client).map_err(ResettingClientError::RequestUnsent))
            .and_then(|client| {
                fixture
                    .wait_for_requests(1)
                    .map_ok(|()| client)
                    .map_err(ResettingClientError::RequestUnobserved)
            })
            .await;

        connected_client.and_then(|client: TcpStream| {
            client
                .set_zero_linger()
                .map_err(ResettingClientError::ZeroLingerUnset)
        })
    }
}
