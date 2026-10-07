use actix_web::web::ServiceConfig;

use paddler_messaging::inference_mode::InferenceMode;

use crate::inference_service::http_route::api::post_continue_from_conversation_history::post_continue_from_conversation_history;
use crate::inference_service::http_route::api::post_continue_from_raw_prompt::post_continue_from_raw_prompt;
use crate::inference_service::http_route::api::post_decide::post_decide;
use crate::inference_service::http_route::api::post_generate_embedding_batch::post_generate_embedding_batch;
use crate::inference_service::http_route::api::ws_inference_socket::ws_inference_socket;

pub fn inference_mode_routes(inference_mode: InferenceMode, service_config: &mut ServiceConfig) {
    match inference_mode {
        InferenceMode::Decision => post_decide(service_config),
        InferenceMode::Embeddings => post_generate_embedding_batch(service_config),
        InferenceMode::TextGeneration => {
            post_continue_from_conversation_history(service_config);
            post_continue_from_raw_prompt(service_config);
            ws_inference_socket(service_config);
        }
    }
}
