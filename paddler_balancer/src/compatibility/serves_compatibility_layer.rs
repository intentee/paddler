use actix_web::web::ServiceConfig;

pub trait ServesCompatibilityLayer: Send + Sync + 'static {
    const EXTRA_ALLOWED_HEADERS: &'static [&'static str];
    const REQUEST_ID_HEADER: &'static str;
    const SERVICE_NAME: &'static str;

    fn configure(service_config: &mut ServiceConfig);
}
