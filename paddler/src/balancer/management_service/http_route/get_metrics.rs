use std::error::Error;

use actix_web::HttpResponse;
use actix_web::Responder;
use actix_web::get;
use actix_web::web::Data;
use actix_web::web::ServiceConfig;
use indoc::formatdoc;

use crate::balancer::agent_controller_pool_total_slots::AgentControllerPoolTotalSlots;
use crate::balancer::management_service::app_data::AppData;

pub fn register(cfg: &mut ServiceConfig) {
    cfg.service(respond);
}

#[get("/metrics")]
async fn respond(app_data: Data<AppData>) -> Result<impl Responder, Box<dyn Error>> {
    let AgentControllerPoolTotalSlots {
        slots_processing,
        slots_total,
    } = app_data.agent_controller_pool.total_slots();
    let buffered_requests_count = app_data
        .buffered_request_manager
        .buffered_request_counter
        .get();
    let statsd_prefix = app_data.statsd_prefix.clone();

    let metrics_response = formatdoc! {"
        # HELP {statsd_prefix}slots_processing Number of processing slots
        # TYPE {statsd_prefix}slots_processing gauge
        {statsd_prefix}slots_processing {slots_processing}

        # HELP {statsd_prefix}slots_total Number of total slots
        # TYPE {statsd_prefix}slots_total gauge
        {statsd_prefix}slots_total {slots_total}

        # HELP {statsd_prefix}requests_buffered Number of buffered requests
        # TYPE {statsd_prefix}requests_buffered gauge
        {statsd_prefix}requests_buffered {buffered_requests_count}
    "};

    Ok(HttpResponse::Ok()
        .content_type("text/plain; version=0.0.4; charset=utf-8; escaping=values")
        .body(metrics_response))
}
