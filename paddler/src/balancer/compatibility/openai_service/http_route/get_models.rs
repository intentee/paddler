use actix_web::HttpResponse;
use actix_web::get;
use actix_web::web;
use serde_json::json;

use crate::balancer::compatibility::openai_service::app_data::AppData;
use crate::balancer::compatibility::openai_service::http_route::current_timestamp;

pub fn register(cfg: &mut web::ServiceConfig) {
    cfg.service(respond);
}

#[get("/v1/models")]
async fn respond(app_data: web::Data<AppData>) -> HttpResponse {
    let created = current_timestamp();

    let model_paths = app_data.buffered_request_manager.get_loaded_model_paths();

    let data: Vec<serde_json::Value> = model_paths
        .iter()
        .map(|path| {
            let model_id = std::path::Path::new(path)
                .file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or(path)
                .to_string();

            json!({
                "id": model_id,
                "object": "model",
                "created": created,
                "owned_by": "paddler"
            })
        })
        .collect();

    HttpResponse::Ok().json(json!({
        "object": "list",
        "data": data
    }))
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;
    use std::sync::Arc;
    use std::time::Duration;

    use actix_web::App;
    use actix_web::test;
    use actix_web::web;
    use tokio::sync::Notify;

    use super::register;
    use crate::balancer::agent_controller_pool::AgentControllerPool;
    use crate::balancer::buffered_request_manager::BufferedRequestManager;
    use crate::balancer::compatibility::openai_service::app_data::AppData;
    use crate::balancer::inference_service::configuration::Configuration;

    fn make_app_data() -> anyhow::Result<AppData> {
        let pool = Arc::new(AgentControllerPool {
            agents: dashmap::DashMap::new(),
            update_notifier: Arc::new(Notify::new()),
        });

        Ok(AppData {
            buffered_request_manager: Arc::new(BufferedRequestManager::new(
                pool,
                Duration::from_secs(30),
                100,
            )),
            inference_service_configuration: Configuration {
                addr: "127.0.0.1:0".parse::<SocketAddr>()?,
                cors_allowed_hosts: vec![],
                inference_item_timeout: Duration::from_secs(30),
            },
        })
    }

    #[actix_web::test]
    async fn get_models_returns_openai_list_shape() -> anyhow::Result<()> {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(make_app_data()?))
                .configure(register),
        )
        .await;

        let req = test::TestRequest::get().uri("/v1/models").to_request();
        let body: serde_json::Value = test::call_and_read_body_json(&app, req).await;

        assert_eq!(body["object"], "list");
        assert!(body["data"].is_array());

        Ok(())
    }
}
