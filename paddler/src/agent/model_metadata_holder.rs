use parking_lot::RwLock;

use crate::model_metadata::ModelMetadata;

pub struct ModelMetadataHolder {
    model_metadata: RwLock<Option<ModelMetadata>>,
}

impl ModelMetadataHolder {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_model_metadata(&self, metadata: ModelMetadata) {
        let mut lock = self.model_metadata.write();

        *lock = Some(metadata);
    }

    pub fn get_model_metadata(&self) -> Option<ModelMetadata> {
        let lock = self.model_metadata.read();

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

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;

    #[test]
    fn new_holder_starts_empty() {
        let holder = ModelMetadataHolder::new();

        assert!(holder.get_model_metadata().is_none());
    }

    #[test]
    fn stored_metadata_is_returned() {
        let holder = ModelMetadataHolder::new();
        let mut metadata = BTreeMap::new();
        metadata.insert("architecture".to_owned(), "llama".to_owned());

        holder.set_model_metadata(ModelMetadata { metadata });

        let stored = holder.get_model_metadata().unwrap();

        assert_eq!(stored.metadata.get("architecture").unwrap(), "llama");
    }
}
