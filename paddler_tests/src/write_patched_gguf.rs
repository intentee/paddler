use std::path::Path;

use llama_cpp_bindings::gguf_context::GgufContext;
use llama_cpp_bindings::gguf_context_error::GgufContextError;

pub fn write_patched_gguf<TPatch>(
    source_path: &Path,
    destination_path: &Path,
    patch: TPatch,
) -> Result<(), GgufContextError>
where
    TPatch: FnOnce(&mut GgufContext),
{
    let mut gguf_context = GgufContext::from_file(source_path)?;

    patch(&mut gguf_context);

    gguf_context.write_to_file(destination_path)
}
