use std::path::Path;

pub fn display_last_path_part(path: &str) -> String {
    Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(path)
        .to_owned()
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use super::display_last_path_part;

    #[test]
    fn returns_only_the_file_name_when_path_contains_separators() -> Result<()> {
        assert_eq!(
            display_last_path_part("/var/models/model.gguf"),
            "model.gguf",
            "expected the file name portion"
        );
        Ok(())
    }

    #[test]
    fn returns_the_input_unchanged_when_there_is_no_separator() -> Result<()> {
        assert_eq!(
            display_last_path_part("just-a-name"),
            "just-a-name",
            "expected the original input when no separator is present"
        );
        Ok(())
    }
}
