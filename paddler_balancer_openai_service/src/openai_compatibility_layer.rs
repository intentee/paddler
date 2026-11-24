use actix_web::web::ServiceConfig;

use paddler_balancer::compatibility::serves_compatibility_layer::ServesCompatibilityLayer;

use crate::constants::OPENAI_HEADER_REQUEST_ID;
use crate::http_route::post_chat_completions::post_chat_completions;
use crate::http_route::post_responses::post_responses;

pub struct OpenAICompatibilityLayer;

impl ServesCompatibilityLayer for OpenAICompatibilityLayer {
    const EXTRA_ALLOWED_HEADERS: &'static [&'static str] = &[];
    const REQUEST_ID_HEADER: &'static str = OPENAI_HEADER_REQUEST_ID;
    const SERVICE_NAME: &'static str = "balancer::compatibility::openai_service";

    fn configure(service_config: &mut ServiceConfig) {
        service_config
            .configure(post_chat_completions)
            .configure(post_responses);
    }
}
