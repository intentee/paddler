use std::net::Ipv4Addr;
use std::net::SocketAddr;

use tokio::io::AsyncWriteExt as _;
use tokio::net::TcpSocket;

use paddler_local_http_fixture::local_http_fixture::LocalHttpFixture;
use paddler_tcp_buffer_limits::tcp_buffer_limits_file::TcpBufferLimitsFile;

const SMALLEST_RECEIVE_BUFFER_REQUEST: u32 = 1;

pub struct ResettingClient {
    receive_buffer: usize,
    socket: TcpSocket,
}

impl ResettingClient {
    pub fn with_the_smallest_receive_buffer() -> Self {
        let socket = TcpSocket::new_v4().expect("the client socket must open");

        socket
            .set_recv_buffer_size(SMALLEST_RECEIVE_BUFFER_REQUEST)
            .expect("the client must shrink its receive buffer");

        Self {
            receive_buffer: usize::try_from(
                socket
                    .recv_buffer_size()
                    .expect("the client must read back its receive buffer"),
            )
            .expect("the receive buffer size must fit in memory"),
            socket,
        }
    }

    pub fn body_outlasting_both_socket_buffers(&self) -> Vec<u8> {
        vec![
            0;
            TcpBufferLimitsFile::of_kernel_send_buffers()
                .largest_buffer()
                .expect("the kernel must report its TCP send buffer limits")
                + self.receive_buffer
                + 1
        ]
    }

    pub async fn request_then_reset(self, fixture: &LocalHttpFixture) {
        let mut client = self
            .socket
            .connect(SocketAddr::from((Ipv4Addr::LOCALHOST, fixture.addr.port())))
            .await
            .expect("the client must connect to the fixture");

        client
            .write_all(b"GET / HTTP/1.1\r\nHost: fixture\r\n\r\n")
            .await
            .expect("the client must send its request");
        fixture
            .wait_for_requests(1)
            .await
            .expect("the fixture must read the request before the client resets");
        client
            .set_zero_linger()
            .expect("the client must reset its connection when it closes");
    }
}
