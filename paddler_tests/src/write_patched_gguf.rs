use std::path::Path;

use llama_cpp_bindings::gguf_context::GgufContext;
use llama_cpp_bindings::gguf_context_error::GgufContextError;

/// # Errors
///
/// Returns the [`GgufContextError`] of a source file that cannot be read, of a patch that fails, and of a destination file that cannot be written.
pub fn write_patched_gguf<TPatch>(
    source_path: &Path,
    destination_path: &Path,
    patch: TPatch,
) -> Result<(), GgufContextError>
where
    TPatch: FnOnce(&mut GgufContext) -> Result<(), GgufContextError>,
{
    GgufContext::from_file(source_path).and_then(|mut gguf_context| {
        patch(&mut gguf_context).and_then(|()| gguf_context.write_to_file(destination_path))
    })
}
