use crate::bound_port::BoundPort;
use crate::check_port::check_port;
use crate::port_check_error::PortCheckError;

pub fn check_optional_port(raw: &str) -> Result<Option<BoundPort>, PortCheckError> {
    if raw.is_empty() {
        return Ok(None);
    }

    check_port(raw).map(Some)
}

#[cfg(test)]
mod tests {
    use super::PortCheckError;
    use super::check_optional_port;

    #[test]
    fn empty_input_resolves_to_no_bound_port() -> anyhow::Result<()> {
        let result = check_optional_port("")?;

        assert!(result.is_none());
        Ok(())
    }

    #[test]
    fn unparseable_input_propagates_the_check_port_error() {
        let result = check_optional_port("not-a-socket-addr");

        assert!(matches!(result, Err(PortCheckError::Unparseable { .. })));
    }

    #[test]
    fn parseable_free_port_resolves_to_some_bound_port() -> anyhow::Result<()> {
        let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
        let address = listener.local_addr()?;
        drop(listener);

        let result = check_optional_port(&address.to_string())?;

        assert!(result.is_some());
        Ok(())
    }
}
