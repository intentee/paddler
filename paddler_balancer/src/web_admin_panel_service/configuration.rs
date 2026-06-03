use std::net::SocketAddr;

use super::template_data::TemplateData;

#[derive(Clone)]
pub struct Configuration {
    pub addr: SocketAddr,
    pub template_data: TemplateData,
}
