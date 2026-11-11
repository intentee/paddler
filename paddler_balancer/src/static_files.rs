use rust_embed::Embed;

#[derive(Embed)]
#[folder = "../static"]
pub struct StaticFiles;

#[cfg(test)]
mod tests {
    use super::StaticFiles;

    #[test]
    fn returns_embedded_file_for_existing_path() {
        let embedded_file = StaticFiles::iter()
            .next()
            .and_then(|file_name| StaticFiles::get(&file_name))
            .unwrap();

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
}
