use rust_embed::Embed;

#[derive(Embed)]
#[folder = "../static"]
pub struct StaticFiles;

#[cfg(test)]
mod tests {
    use std::hint::black_box;

    use rust_embed::EmbeddedFile;

    use super::StaticFiles;

    fn any_embedded_file_name() -> String {
        StaticFiles::iter()
            .next()
            .map(|file_name| file_name.as_ref().to_owned())
            .unwrap()
    }

    #[test]
    fn returns_embedded_file_for_existing_path() {
        let embedded_file = StaticFiles::get(&any_embedded_file_name()).unwrap();

        assert!(!embedded_file.data.is_empty());
    }

    #[test]
    fn returns_none_for_missing_path() {
        assert!(StaticFiles::get("this_file_does_not_exist.txt").is_none());
    }

    #[test]
    fn returns_none_for_path_traversal_outside_embedded_folder() {
        assert!(StaticFiles::get("../Cargo.toml").is_none());
    }

    #[test]
    fn iterates_over_embedded_file_names() {
        assert!(StaticFiles::iter().next().is_some());
    }

    #[test]
    fn returns_embedded_file_when_called_through_indirect_call() {
        let lookup: fn(&str) -> Option<EmbeddedFile> = black_box(StaticFiles::get);
        let embedded_file = lookup(&any_embedded_file_name()).unwrap();

        assert!(!embedded_file.data.is_empty());
    }
}
