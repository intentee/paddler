use llama_cpp_bindings::mtmd::MtmdDefaultMarkerError;

pub trait ProvidesMediaMarker: Send + 'static {
    fn media_marker(&self) -> Result<&'static str, MtmdDefaultMarkerError>;
}
