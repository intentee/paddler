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
    GgufContext::from_file(source_path).and_then(|mut gguf_context| {
        patch(&mut gguf_context);

        gguf_context.write_to_file(destination_path)
    })
}
