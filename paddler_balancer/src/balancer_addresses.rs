use std::io;
use std::io::Write;
use std::net::SocketAddr;

use serde::Deserialize;
use serde::Serialize;
use serde_json::to_writer;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BalancerAddresses {
    pub compat_openai: Option<SocketAddr>,
    pub inference: SocketAddr,
    pub management: SocketAddr,
    pub web_admin_panel: Option<SocketAddr>,
}

impl BalancerAddresses {
    pub fn write_json_line<TWriter: Write>(&self, mut writer: TWriter) -> io::Result<()> {
        to_writer(&mut writer, self)?;

        writeln!(writer)
    }
}

#[cfg(test)]
mod tests {
    use std::io::ErrorKind;
    use std::net::Ipv4Addr;
    use std::net::SocketAddr;

    use super::BalancerAddresses;

    fn inference_and_management_only() -> BalancerAddresses {
        BalancerAddresses {
            compat_openai: None,
            inference: SocketAddr::from((Ipv4Addr::LOCALHOST, 8061)),
            management: SocketAddr::from((Ipv4Addr::LOCALHOST, 8060)),
            web_admin_panel: None,
        }
    }

    #[test]
    fn writes_itself_as_a_single_json_line() {
        let mut announcement = Vec::new();

        inference_and_management_only()
            .write_json_line(&mut announcement)
            .expect("writing into a growable buffer must succeed");

        assert_eq!(
            String::from_utf8(announcement).expect("the announcement must be UTF-8"),
            "{\"compat_openai\":null,\"inference\":\"127.0.0.1:8061\",\"management\":\"127.0.0.1:8060\",\"web_admin_panel\":null}\n"
        );
    }

    #[test]
    fn reports_a_writer_that_cannot_hold_the_announcement() {
        let zero_capacity_buffer: &mut [u8] = &mut [];

        let write_error = inference_and_management_only()
            .write_json_line(zero_capacity_buffer)
            .expect_err("a zero-capacity writer must reject the announcement");

        assert_eq!(write_error.kind(), ErrorKind::WriteZero);
    }
}
