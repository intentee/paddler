use std::sync::RwLock;

use crate::model_metadata::ModelMetadata;

pub struct ModelMetadataHolder {
    model_metadata: RwLock<Option<ModelMetadata>>,
}

impl ModelMetadataHolder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_model_metadata(&self, metadata: ModelMetadata) {
        let mut lock = self
            .model_metadata
            .write()
            .expect("Failed to acquire write lock on model metadata");

        *lock = Some(metadata);
    }

    pub fn get_model_metadata(&self) -> Option<ModelMetadata> {
        let lock = self
            .model_metadata
            .read()
            .expect("Failed to acquire read lock on model metadata");

        lock.clone()
    }
}

impl Default for ModelMetadataHolder {
    fn default() -> Self {
        Self {
            model_metadata: RwLock::new(None),
        }
    }
}
