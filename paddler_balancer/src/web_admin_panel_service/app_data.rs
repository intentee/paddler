use std::sync::Arc;

use esbuild_metafile::EsbuildMetaFile;

use crate::web_admin_panel_service::template_data::TemplateData;

pub struct AppData {
    pub esbuild_metafile: Arc<EsbuildMetaFile>,
    pub template_data: TemplateData,
}
