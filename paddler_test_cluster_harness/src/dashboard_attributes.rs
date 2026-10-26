use std::collections::BTreeMap;

use quick_xml::Reader;
use quick_xml::XmlVersion;
use quick_xml::events::BytesStart;
use quick_xml::events::Event;

use crate::cluster_harness_error::ClusterHarnessError;

const DASHBOARD_ELEMENT_ID: &str = "paddler-dashboard";

fn attributes_of(
    element: &BytesStart<'_>,
) -> Result<BTreeMap<String, String>, ClusterHarnessError> {
    element
        .html_attributes()
        .map(|attribute_result| {
            let attribute =
                attribute_result.map_err(ClusterHarnessError::DashboardAttributeMalformed)?;
            let value = attribute
                .normalized_value(XmlVersion::Implicit1_0)
                .map_err(ClusterHarnessError::DashboardAttributeValueUnreadable)?;

            Ok((attribute.key.as_ref().to_owned(), value.into_owned()))
        })
        .collect()
}

pub fn dashboard_attributes(page: &str) -> Result<BTreeMap<String, String>, ClusterHarnessError> {
    let mut reader = Reader::from_str(page);

    reader.config_mut().check_end_names = false;

    loop {
        match reader
            .read_event()
            .map_err(ClusterHarnessError::DashboardMarkupUnreadable)?
        {
            Event::Start(element) | Event::Empty(element) => {
                let attributes = attributes_of(&element)?;

                if attributes
                    .get("id")
                    .is_some_and(|element_id| element_id == DASHBOARD_ELEMENT_ID)
                {
                    return Ok(attributes);
                }
            }
            Event::Eof => return Err(ClusterHarnessError::DashboardElementMissing),
            _other_event => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::error::Error as _;
    use std::mem::discriminant;

    use quick_xml::errors::Error as XmlError;
    use quick_xml::errors::SyntaxError;
    use quick_xml::escape::EscapeError;
    use quick_xml::events::attributes::AttrError;

    use super::dashboard_attributes;
    use crate::cluster_harness_error::ClusterHarnessError;

    #[test]
    fn a_self_closing_dashboard_element_carries_its_attributes() {
        assert_eq!(
            dashboard_attributes(
                r#"<html><body><div id="paddler-dashboard" data-buffered-requests="7"/></body></html>"#
            )
            .unwrap(),
            BTreeMap::from([
                ("data-buffered-requests".to_owned(), "7".to_owned()),
                ("id".to_owned(), "paddler-dashboard".to_owned()),
            ])
        );
    }

    #[test]
    fn markup_ending_inside_a_tag_is_unreadable() {
        let unreadable_markup =
            dashboard_attributes(r#"<html><body><div id="paddler-dashboard""#).unwrap_err();

        assert_eq!(
            discriminant(&unreadable_markup),
            discriminant(&ClusterHarnessError::DashboardMarkupUnreadable(
                XmlError::Syntax(SyntaxError::UnclosedTag)
            ))
        );
        assert_eq!(
            unreadable_markup
                .source()
                .and_then(|xml_error| xml_error.source())
                .and_then(|syntax_error| syntax_error.downcast_ref::<SyntaxError>()),
            Some(&SyntaxError::UnclosedTag)
        );
    }

    #[test]
    fn a_repeated_attribute_is_malformed() {
        let malformed_attribute =
            dashboard_attributes(r#"<div id="paddler-dashboard" id="another-dashboard">"#)
                .unwrap_err();

        assert_eq!(
            discriminant(&malformed_attribute),
            discriminant(&ClusterHarnessError::DashboardAttributeMalformed(
                AttrError::Duplicated(27, 4)
            ))
        );
        assert_eq!(
            malformed_attribute
                .source()
                .and_then(|attribute_error| attribute_error.downcast_ref::<AttrError>()),
            Some(&AttrError::Duplicated(27, 4))
        );
    }

    #[test]
    fn an_attribute_referencing_an_unknown_entity_is_unreadable() {
        assert!(matches!(
            dashboard_attributes(r#"<div id="paddler-dashboard" data-version="&unknown;">"#),
            Err(ClusterHarnessError::DashboardAttributeValueUnreadable(XmlError::Escape(
                EscapeError::UnrecognizedEntity(_, entity)
            ))) if entity == "unknown"
        ));
    }

    #[test]
    fn a_page_without_the_dashboard_reports_it_missing() {
        assert_eq!(
            discriminant(&dashboard_attributes("<html><body></body></html>").unwrap_err()),
            discriminant(&ClusterHarnessError::DashboardElementMissing)
        );
    }
}
