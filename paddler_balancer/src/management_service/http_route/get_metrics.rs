use actix_web::HttpResponse;
use actix_web::Responder;
use actix_web::web::Data;
use actix_web::web::ServiceConfig;
use actix_web::web::get;

use paddler_messaging::api_path::ApiPath;

use crate::balancer_metrics::BalancerMetrics;
use crate::management_service::app_data::AppData;

async fn respond(app_data: Data<AppData>) -> impl Responder {
    let balancer_metrics = BalancerMetrics::gather(
        &app_data.agent_controller_pool,
        &app_data.buffered_request_manager,
    );

    HttpResponse::Ok()
        .content_type("text/plain; version=0.0.4; charset=utf-8; escaping=values")
        .body(balancer_metrics.to_prometheus_text(&app_data.statsd_prefix))
}

pub fn get_metrics(cfg: &mut ServiceConfig) {
    cfg.route(ApiPath::METRICS, get().to(respond));
}
