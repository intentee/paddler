use syn::Attribute;
use syn::Ident;
use syn::ItemMod;
use syn::visit::Visit;
use syn::visit::visit_item_mod;

#[derive(Default)]
pub struct TestOnlyAttributeCounter {
    pub test_only_attributes: usize,
}

impl<'ast> Visit<'ast> for TestOnlyAttributeCounter {
    fn visit_attribute(&mut self, attribute: &'ast Attribute) {
        if attribute.path().is_ident("cfg")
            && attribute
                .parse_args::<Ident>()
                .is_ok_and(|cfg_predicate| cfg_predicate == "test")
        {
            self.test_only_attributes += 1;
        }
    }

    fn visit_item_mod(&mut self, item_mod: &'ast ItemMod) {
        if item_mod.ident != "tests" || item_mod.content.is_none() {
            visit_item_mod(self, item_mod);
        }
    }
}
