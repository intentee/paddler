use paddler_messaging::model_metadata::ModelMetadata;

#[derive(Clone, Debug)]
pub enum LoadedModelMetadata {
    Loaded(ModelMetadata),
    NoModelLoaded,
}
