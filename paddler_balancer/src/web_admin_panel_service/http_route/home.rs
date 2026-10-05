use actix_web::Responder;
use actix_web::web;
use actix_web::web::get;
use askama::Template;
use esbuild_metafile::HttpPreloader;
use esbuild_metafile::filters;

use crate::response::view::view;
use crate::web_admin_panel_service::app_data::AppData;
use crate::web_admin_panel_service::template_data::TemplateData;

async fn respond(app_data: web::Data<AppData>) -> impl Responder {
    view(WebAdminPanelTemplate {
        preloads: HttpPreloader::new(app_data.esbuild_metafile.clone()),
        template_data: &app_data.template_data,
    })
}

#[derive(Template)]
#[template(path = "web_admin_panel.html")]
struct WebAdminPanelTemplate<'template_data> {
    preloads: HttpPreloader,
    template_data: &'template_data TemplateData,
}

pub fn home(cfg: &mut web::ServiceConfig) {
    cfg.route("/{_:.*}", get().to(respond));
}
