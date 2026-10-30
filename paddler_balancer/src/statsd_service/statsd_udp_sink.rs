use std::io;
use std::net::SocketAddr;
use std::net::UdpSocket;

use cadence::MetricSink;

pub struct StatsdUdpSink {
    pub socket: UdpSocket,
    pub statsd_addr: SocketAddr,
}

impl MetricSink for StatsdUdpSink {
    fn emit(&self, metric: &str) -> io::Result<usize> {
        self.socket.send_to(metric.as_bytes(), self.statsd_addr)
    }
}
