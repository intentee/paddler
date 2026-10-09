use std::fmt::Display;

pub fn rejection_description<TRejection: Display>(
    agent_name: Option<&str>,
    rejection: &TRejection,
) -> String {
    agent_name.map_or_else(
        || rejection.to_string(),
        |agent_name| format!("{agent_name}: {rejection}"),
    )
}

#[cfg(test)]
mod tests {
    use super::rejection_description;

    #[test]
    fn prefixes_the_description_with_the_agent_name() {
        assert_eq!(
            rejection_description(Some("agent"), &"no model is loaded"),
            "agent: no model is loaded"
        );
    }

    #[test]
    fn describes_the_rejection_alone_for_an_unnamed_agent() {
        assert_eq!(
            rejection_description(None, &"no model is loaded"),
            "no model is loaded"
        );
    }
}
