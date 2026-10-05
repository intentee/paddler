use minijinja::Error;
use minijinja::ErrorKind;

// Surfaces errors raised explicitly inside a chat template. Known uses:
// https://huggingface.co/bartowski/Mistral-7B-Instruct-v0.3-GGUF
pub fn raise_exception(message: &str) -> Result<String, Error> {
    Err(Error::new::<String>(
        ErrorKind::InvalidOperation,
        format!("Model's chat template raised an exception: '{message}'"),
    ))
}

#[cfg(test)]
mod tests {
    use minijinja::ErrorKind;

    use super::raise_exception;

    #[test]
    fn returns_err_with_supplied_message_quoted() {
        let error = raise_exception("template is invalid")
            .expect_err("raise_exception must always return Err");
        let rendered = error.to_string();

        assert!(
            rendered.contains("template is invalid"),
            "error must include the supplied message; got: {rendered}"
        );
    }

    #[test]
    fn returns_err_with_invalid_operation_kind() {
        let error =
            raise_exception("anything").expect_err("raise_exception must always return Err");

        assert_eq!(error.kind(), ErrorKind::InvalidOperation);
    }
}
