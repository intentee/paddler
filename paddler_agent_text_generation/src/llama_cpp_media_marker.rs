use llama_cpp_bindings::mtmd::MtmdDefaultMarkerError;
use llama_cpp_bindings::mtmd::mtmd_default_marker;

use crate::provides_media_marker::ProvidesMediaMarker;

pub struct LlamaCppMediaMarker;

impl ProvidesMediaMarker for LlamaCppMediaMarker {
    fn media_marker(&self) -> Result<&'static str, MtmdDefaultMarkerError> {
        mtmd_default_marker()
    }
}
