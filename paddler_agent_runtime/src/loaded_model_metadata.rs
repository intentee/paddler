use std::sync::Arc;

use paddler_messaging::model_metadata::ModelMetadata;

#[derive(Clone, Debug)]
pub enum LoadedModelMetadata {
    Loaded(Arc<ModelMetadata>),
    NoModelLoaded,
}
