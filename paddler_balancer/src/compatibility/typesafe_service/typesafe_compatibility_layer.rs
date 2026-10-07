use actix_web::web::ServiceConfig;

use crate::compatibility::serves_compatibility_layer::ServesCompatibilityLayer;
use crate::compatibility::typesafe_service::http_route::get_models::get_models;
use crate::compatibility::typesafe_service::http_route::post_system_one::post_system_one;
use crate::compatibility::typesafe_service::typesafe_header::TypeSafeHeader;

pub struct TypeSafeCompatibilityLayer;

impl ServesCompatibilityLayer for TypeSafeCompatibilityLayer {
    const EXTRA_ALLOWED_HEADERS: &'static [&'static str] = &[
        TypeSafeHeader::RETRY_COUNT,
        TypeSafeHeader::RUNTIME,
        TypeSafeHeader::SDK,
    ];
    const REQUEST_ID_HEADER: &'static str = TypeSafeHeader::REQUEST_ID;
    const SERVICE_NAME: &'static str = "balancer::compatibility::typesafe_service";

    fn configure(service_config: &mut ServiceConfig) {
        service_config
            .configure(get_models)
            .configure(post_system_one);
    }
}
