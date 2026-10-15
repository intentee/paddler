use serde::Serialize;

const RELEASE_DATE_UNKNOWN: &str = "unknown";

#[derive(Serialize)]
pub struct TypeSafeModel {
    pub name: String,
    pub description: String,
    pub release_date: String,
}

impl TypeSafeModel {
    #[must_use]
    pub fn named(name: String) -> Self {
        Self {
            description: format!("Decision model {name} served by Paddler"),
            name,
            release_date: RELEASE_DATE_UNKNOWN.to_owned(),
        }
    }
}
