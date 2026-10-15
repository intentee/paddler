use std::str::from_utf8;

use llama_cpp_bindings::mtmd::MtmdDefaultMarkerError;

use paddler_agent_text_generation::provides_media_marker::ProvidesMediaMarker;

const fn bytes_that_are_not_utf8() -> &'static [u8] {
    &[u8::MAX]
}

pub struct MediaMarkerThatIsNotUtf8;

impl ProvidesMediaMarker for MediaMarkerThatIsNotUtf8 {
    fn media_marker(&self) -> Result<&'static str, MtmdDefaultMarkerError> {
        from_utf8(bytes_that_are_not_utf8()).map_err(MtmdDefaultMarkerError::NotUtf8)
    }
}
