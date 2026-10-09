use std::io;
use std::net::SocketAddr;
use std::net::TcpListener;

use socket2::Domain;
use socket2::Protocol;
use socket2::Socket;
use socket2::Type;

const ACTIX_WEB_DEFAULT_BACKLOG: i32 = 1024;

pub struct HttpListener {
    pub local_addr: SocketAddr,
    pub tcp_listener: TcpListener,
}

impl HttpListener {
    pub fn bind(addr: SocketAddr) -> io::Result<Self> {
        let socket = Socket::new(Domain::for_address(addr), Type::STREAM, Some(Protocol::TCP))?;

        socket.set_reuse_address(true)?;

        socket.bind(&addr.into())?;
        socket.listen(ACTIX_WEB_DEFAULT_BACKLOG)?;

        let tcp_listener = TcpListener::from(socket);
        let local_addr = tcp_listener.local_addr()?;

        Ok(Self {
            local_addr,
            tcp_listener,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::io::ErrorKind;
    use std::net::Ipv4Addr;
    use std::net::SocketAddr;
    use std::net::TcpListener;
    use std::net::TcpStream;

    use super::HttpListener;

    #[test]
    fn accepts_connections_on_the_port_it_bound_for_port_zero() {
        let http_listener = HttpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, 0)))
            .expect("an ephemeral loopback port must be bindable");

        assert_ne!(http_listener.local_addr.port(), 0);

        TcpStream::connect(http_listener.local_addr)
            .expect("a listening socket must accept a connection");
    }

    #[test]
    fn fails_when_the_address_is_already_listened_on() {
        let occupying_listener = TcpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, 0)))
            .expect("an ephemeral loopback port must be bindable");
        let taken_addr = occupying_listener
            .local_addr()
            .expect("a bound listener must report its address");

        let bind_error = HttpListener::bind(taken_addr)
            .err()
            .expect("binding an address another socket listens on must fail");

        assert_eq!(bind_error.kind(), ErrorKind::AddrInUse);
    }
}
