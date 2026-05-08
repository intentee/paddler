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
    use anyhow::Result;
    use anyhow::anyhow;

    use super::raise_exception;

    #[test]
    fn returns_err_with_supplied_message_quoted() -> Result<()> {
        let err = raise_exception("template is invalid")
            .err()
            .ok_or_else(|| anyhow!("expected Err, got Ok"))?;
        let rendered = err.to_string();

        if !rendered.contains("template is invalid") {
            return Err(anyhow!(
                "error must include the supplied message; got: {rendered}"
            ));
        }

        Ok(())
    }

    #[test]
    fn returns_err_with_invalid_operation_kind() -> Result<()> {
        let err = raise_exception("anything")
            .err()
            .ok_or_else(|| anyhow!("expected Err, got Ok"))?;

        assert_eq!(err.kind(), minijinja::ErrorKind::InvalidOperation);

        Ok(())
    }
}
