use syn::parse_file;
use syn::visit::Visit as _;

use crate::test_only_attribute_counter::TestOnlyAttributeCounter;

#[test]
fn the_test_only_code_detector_counts_only_test_code_outside_inline_test_modules() {
    let mut test_only_attribute_counter = TestOnlyAttributeCounter::default();

    test_only_attribute_counter.visit_file(
        &parse_file(
            r#"
                #[cfg(test)]
                mod helper_outside_the_inline_test_module;

                #[cfg(feature = "tests_that_use_llms")]
                fn feature_gated_production_code() {}

                #[cfg(test)]
                mod tests {
                    #[cfg(test)]
                    fn helper_inside_the_inline_test_module() {}
                }
            "#,
        )
        .expect("the probe source must parse"),
    );

    assert_eq!(test_only_attribute_counter.test_only_attributes, 1);
}
