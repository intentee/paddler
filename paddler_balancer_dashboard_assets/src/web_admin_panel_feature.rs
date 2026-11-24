use std::ffi::OsString;

#[derive(Debug, Eq, PartialEq)]
pub enum WebAdminPanelFeature {
    Disabled,
    Enabled,
}

impl WebAdminPanelFeature {
    #[must_use]
    pub const fn from_cargo_feature_variable(cargo_feature_variable: Option<&OsString>) -> Self {
        match cargo_feature_variable {
            Some(_) => Self::Enabled,
            None => Self::Disabled,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;

    use super::WebAdminPanelFeature;

    #[test]
    fn a_set_cargo_feature_variable_enables_the_web_admin_panel() {
        assert_eq!(
            WebAdminPanelFeature::from_cargo_feature_variable(Some(&OsString::from("1"))),
            WebAdminPanelFeature::Enabled
        );
    }

    #[test]
    fn an_unset_cargo_feature_variable_disables_the_web_admin_panel() {
        assert_eq!(
            WebAdminPanelFeature::from_cargo_feature_variable(None),
            WebAdminPanelFeature::Disabled
        );
    }
}
