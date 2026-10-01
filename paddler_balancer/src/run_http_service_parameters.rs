use crate::http_listener::HttpListener;

pub struct RunHttpServiceParameters<TAppFactory> {
    pub app_factory: TAppFactory,
    pub http_listener: HttpListener,
    pub service_name: &'static str,
}
