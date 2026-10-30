use std::fs::read_dir;
use std::fs::read_to_string;
use std::path::Path;
use std::path::PathBuf;

use syn::parse_file;
use syn::visit::Visit as _;

use crate::test_only_attribute_counter::TestOnlyAttributeCounter;

#[test]
fn production_sources_keep_test_code_in_inline_test_modules() {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the test crate must live inside the workspace");
    let mut source_directories: Vec<PathBuf> = read_dir(workspace_root)
        .expect("the workspace root must be listable")
        .map(|entry| entry.expect("a workspace entry must be readable").path())
        .filter(|crate_directory| crate_directory.join("Cargo.toml").is_file())
        .map(|crate_directory| crate_directory.join("src"))
        .filter(|source_directory| source_directory.is_dir())
        .collect();
    let mut rust_sources = Vec::new();

    while let Some(source_directory) = source_directories.pop() {
        let source_entries: Vec<PathBuf> = read_dir(source_directory)
            .expect("a source directory must be listable")
            .map(|entry| entry.expect("a source entry must be readable").path())
            .collect();

        source_directories.extend(source_entries.iter().filter(|path| path.is_dir()).cloned());
        rust_sources.extend(
            source_entries
                .into_iter()
                .filter(|path| path.extension().is_some_and(|extension| extension == "rs")),
        );
    }

    let sources_with_test_only_code_outside_inline_test_modules: Vec<PathBuf> = rust_sources
        .into_iter()
        .filter(|rust_source| {
            let mut test_only_attribute_counter = TestOnlyAttributeCounter::default();

            test_only_attribute_counter.visit_file(
                &parse_file(&read_to_string(rust_source).expect("a source file must be readable"))
                    .expect("a source file must parse"),
            );

            test_only_attribute_counter.test_only_attributes > 0
        })
        .collect();

    assert_eq!(
        sources_with_test_only_code_outside_inline_test_modules,
        Vec::<PathBuf>::new()
    );
}
