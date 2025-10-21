use actix_web::Responder;
use actix_web::get;
use actix_web::web;
use askama::Template;
use esbuild_metafile::HttpPreloader;
use esbuild_metafile::filters;

use crate::balancer::response::view;
use crate::balancer::web_admin_panel_service::app_data::AppData;

pub fn register(cfg: &mut web::ServiceConfig) {
    cfg.service(respond);
}

#[derive(Template)]
#[template(path = "web_admin_panel.html")]
struct WebAdminPanelTemplate {
    buffered_request_timeout_millis: u128,
    compat_openai_addr: String,
    inference_addr: String,
    management_addr: String,
    max_buffered_requests: i32,
    preloads: HttpPreloader,
    statsd_addr: String,
    statsd_prefix: String,
    statsd_reporting_interval_millis: u128,
}

#[get("/{_:.*}")]
async fn respond(preloads: HttpPreloader, app_data: web::Data<AppData>) -> impl Responder {
    view(WebAdminPanelTemplate {
        buffered_request_timeout_millis: app_data
            .template_data
            .buffered_request_timeout
            .as_millis(),
        compat_openai_addr: match app_data.template_data.compat_openai_addr.clone() {
            Some(addr) => addr.input_addr.clone(),
            None => String::new(),
        },
        inference_addr: app_data.template_data.inference_addr.input_addr.clone(),
        management_addr: app_data.template_data.management_addr.input_addr.clone(),
        max_buffered_requests: app_data.template_data.max_buffered_requests,
        preloads,
        statsd_addr: match app_data.template_data.statsd_addr.clone() {
            Some(addr) => addr.input_addr.clone(),
            None => String::new(),
        },
        statsd_prefix: app_data.template_data.statsd_prefix.clone(),
        statsd_reporting_interval_millis: app_data
            .template_data
            .statsd_reporting_interval
            .as_millis(),
    })
}
