use std::error::Error;
use std::sync::atomic::Ordering;

use actix_web::{get, web, HttpResponse, Responder};
use indoc::indoc;

use crate::balancer::upstream_peer_pool::UpstreamPeerPool;

pub fn register(cfg: &mut web::ServiceConfig) {
    cfg.service(respond);
}

#[get("/api/v1/metrics")]
async fn respond(
    upstream_peer_pool: web::Data<UpstreamPeerPool>,
) -> Result<impl Responder, Box<dyn Error>> {
    let (slots_idle, slots_processing) = upstream_peer_pool.total_slots()?;
    let requests_buffered = upstream_peer_pool
        .request_buffer_length
        .load(Ordering::SeqCst);

    let metrics_response = format!(
        indoc! {"
# HELP paddler_slots_idle Number of idle slots
# TYPE paddler_slots_idle gauge
paddler_slots_idle {}

# HELP paddler_slots_processing Number of processing slots
# TYPE paddler_slots_processing gauge
paddler_slots_processing {}

# HELP paddler_requests_buffered Number of buffered requests
# TYPE paddler_requests_buffered gauge
paddler_requests_buffered {}
"},
        slots_idle, slots_processing, requests_buffered
    );

    Ok(HttpResponse::Ok()
        .content_type("text/plain; version=0.0.4; charset=utf-8; escaping=values")
        .body(metrics_response))
}
