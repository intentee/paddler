use actix_web::Error;
use actix_web::body::MessageBody;
use actix_web::dev::ServiceRequest;
use actix_web::dev::ServiceResponse;
use actix_web::http::header::HeaderName;
use actix_web::http::header::HeaderValue;
use actix_web::middleware::Next;
use rand::random;

use crate::compatibility::serves_compatibility_layer::ServesCompatibilityLayer;

pub async fn attach_request_id<TCompatibilityLayer: ServesCompatibilityLayer>(
    request: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<impl MessageBody>, Error> {
    let request_id = request
        .headers()
        .get(TCompatibilityLayer::REQUEST_ID_HEADER)
        .cloned()
        .unwrap_or_else(|| HeaderValue::from(random::<u64>()));

    next.call(request).await.map(|mut response| {
        response.headers_mut().insert(
            HeaderName::from_static(TCompatibilityLayer::REQUEST_ID_HEADER),
            request_id,
        );

        response
    })
}
