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
            Some(addr) => addr.input_addr,
            None => String::new(),
        },
        inference_addr: app_data.template_data.inference_addr.input_addr.clone(),
        management_addr: app_data.template_data.management_addr.input_addr.clone(),
        max_buffered_requests: app_data.template_data.max_buffered_requests,
        preloads,
        statsd_addr: match app_data.template_data.statsd_addr.clone() {
            Some(addr) => addr.input_addr,
            None => String::new(),
        },
        statsd_prefix: app_data.template_data.statsd_prefix.clone(),
        statsd_reporting_interval_millis: app_data
            .template_data
            .statsd_reporting_interval
            .as_millis(),
    })
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;
    use std::time::Duration;

    use actix_web::App;
    use actix_web::http::StatusCode;
    use actix_web::test;
    use actix_web::web::Data;
    use parking_lot::Once;

    use super::register;
    use crate::balancer::web_admin_panel_service::app_data::AppData;
    use crate::balancer::web_admin_panel_service::template_data::TemplateData;
    use crate::resolved_socket_addr::ResolvedSocketAddr;

    static INIT_ESBUILD_METAFILE: Once = Once::new();

    fn ensure_esbuild_metafile_initialized() {
        INIT_ESBUILD_METAFILE.call_once(|| {
            esbuild_metafile::instance::initialize_instance(include_str!(
                "../../../../../esbuild-meta.json"
            ));
        });
    }

    #[actix_web::test]
    async fn renders_optional_addresses_when_present() {
        ensure_esbuild_metafile_initialized();

        let app_data = Data::new(AppData {
            template_data: TemplateData {
                buffered_request_timeout: Duration::from_secs(1),
                compat_openai_addr: Some(ResolvedSocketAddr {
                    input_addr: "127.0.0.1:8081".to_owned(),
                    socket_addr: SocketAddr::from(([127, 0, 0, 1], 8081)),
                }),
                inference_addr: ResolvedSocketAddr {
                    input_addr: "127.0.0.1:8082".to_owned(),
                    socket_addr: SocketAddr::from(([127, 0, 0, 1], 8082)),
                },
                management_addr: ResolvedSocketAddr {
                    input_addr: "127.0.0.1:8083".to_owned(),
                    socket_addr: SocketAddr::from(([127, 0, 0, 1], 8083)),
                },
                max_buffered_requests: 32,
                statsd_addr: Some(ResolvedSocketAddr {
                    input_addr: "127.0.0.1:8125".to_owned(),
                    socket_addr: SocketAddr::from(([127, 0, 0, 1], 8125)),
                }),
                statsd_prefix: "paddler".to_owned(),
                statsd_reporting_interval: Duration::from_millis(500),
            },
        });
        let app = test::init_service(App::new().app_data(app_data).configure(register)).await;
        let request = test::TestRequest::get().uri("/").to_request();
        let response = test::call_service(&app, request).await;

        assert_eq!(response.status(), StatusCode::OK);

        let body = test::read_body(response).await;
        let body_text = std::str::from_utf8(body.as_ref()).unwrap();

        assert!(body_text.contains("data-compat-openai-addr=\"127.0.0.1:8081\""));
        assert!(body_text.contains("data-statsd-addr=\"127.0.0.1:8125\""));
        assert!(body_text.contains("data-inference-addr=\"127.0.0.1:8082\""));
        assert!(body_text.contains("data-management-addr=\"127.0.0.1:8083\""));
        assert!(body_text.contains("data-buffered-request-timeout-millis=\"1000\""));
        assert!(body_text.contains("data-max-buffered-requests=\"32\""));
        assert!(body_text.contains("data-statsd-prefix=\"paddler\""));
        assert!(body_text.contains("data-statsd-reporting-interval-millis=\"500\""));
    }

    #[actix_web::test]
    async fn renders_empty_addresses_when_absent() {
        ensure_esbuild_metafile_initialized();

        let app_data = Data::new(AppData {
            template_data: TemplateData {
                buffered_request_timeout: Duration::from_secs(1),
                compat_openai_addr: None,
                inference_addr: ResolvedSocketAddr {
                    input_addr: "127.0.0.1:8082".to_owned(),
                    socket_addr: SocketAddr::from(([127, 0, 0, 1], 8082)),
                },
                management_addr: ResolvedSocketAddr {
                    input_addr: "127.0.0.1:8083".to_owned(),
                    socket_addr: SocketAddr::from(([127, 0, 0, 1], 8083)),
                },
                max_buffered_requests: 32,
                statsd_addr: None,
                statsd_prefix: "paddler".to_owned(),
                statsd_reporting_interval: Duration::from_millis(500),
            },
        });
        let app = test::init_service(App::new().app_data(app_data).configure(register)).await;
        let request = test::TestRequest::get().uri("/").to_request();
        let response = test::call_service(&app, request).await;

        assert_eq!(response.status(), StatusCode::OK);

        let body = test::read_body(response).await;
        let body_text = std::str::from_utf8(body.as_ref()).unwrap();

        assert!(body_text.contains("data-compat-openai-addr=\"\""));
        assert!(body_text.contains("data-statsd-addr=\"\""));
    }
}
