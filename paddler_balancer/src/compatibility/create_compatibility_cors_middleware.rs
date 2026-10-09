use std::sync::Arc;

use actix_cors::Cors;

use crate::compatibility::serves_compatibility_layer::ServesCompatibilityLayer;
use crate::create_cors_middleware::create_cors_middleware;

pub fn create_compatibility_cors_middleware<TCompatibilityLayer: ServesCompatibilityLayer>(
    allowed_hosts: &Arc<Vec<String>>,
) -> Cors {
    create_cors_middleware(allowed_hosts)
        .allowed_headers(
            TCompatibilityLayer::EXTRA_ALLOWED_HEADERS
                .iter()
                .copied()
                .chain([TCompatibilityLayer::REQUEST_ID_HEADER]),
        )
        .expose_headers([TCompatibilityLayer::REQUEST_ID_HEADER])
}
