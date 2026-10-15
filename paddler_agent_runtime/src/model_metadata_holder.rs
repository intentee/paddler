use parking_lot::RwLock;

use paddler_messaging::model_metadata::ModelMetadata;

use crate::loaded_model_metadata::LoadedModelMetadata;

pub struct ModelMetadataHolder {
    loaded_model_metadata: RwLock<LoadedModelMetadata>,
}

impl ModelMetadataHolder {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_model_metadata(&self, model_metadata: ModelMetadata) {
        *self.loaded_model_metadata.write() = LoadedModelMetadata::Loaded(model_metadata);
    }

    pub fn forget_model_metadata(&self) {
        *self.loaded_model_metadata.write() = LoadedModelMetadata::NoModelLoaded;
    }

    #[must_use]
    pub fn get_model_metadata(&self) -> Option<ModelMetadata> {
        match &*self.loaded_model_metadata.read() {
            LoadedModelMetadata::Loaded(model_metadata) => Some(model_metadata.clone()),
            LoadedModelMetadata::NoModelLoaded => None,
        }
    }
}

impl Default for ModelMetadataHolder {
    fn default() -> Self {
        Self {
            loaded_model_metadata: RwLock::new(LoadedModelMetadata::NoModelLoaded),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use paddler_messaging::model_metadata::ModelMetadata;

    use super::ModelMetadataHolder;

    fn llama_metadata() -> ModelMetadata {
        ModelMetadata {
            metadata: BTreeMap::from([("architecture".to_owned(), "llama".to_owned())]),
        }
    }

    #[test]
    fn new_holder_starts_empty() {
        assert_eq!(ModelMetadataHolder::new().get_model_metadata(), None);
    }

    #[test]
    fn stored_metadata_is_returned() {
        let holder = ModelMetadataHolder::new();

        holder.set_model_metadata(llama_metadata());

        assert_eq!(holder.get_model_metadata(), Some(llama_metadata()));
    }

    #[test]
    fn forgotten_metadata_is_no_longer_returned() {
        let holder = ModelMetadataHolder::new();

        holder.set_model_metadata(llama_metadata());
        holder.forget_model_metadata();

        assert_eq!(holder.get_model_metadata(), None);
    }
}
