use std::collections::BTreeMap;
use std::str::from_utf8;

use quick_xml::Reader;
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
            let name = from_utf8(attribute.key.as_ref())
                .map_err(ClusterHarnessError::DashboardAttributeNameNotUtf8)?;
            let value = attribute
                .unescape_value()
                .map_err(ClusterHarnessError::DashboardAttributeValueUnreadable)?;

            Ok((name.to_owned(), value.into_owned()))
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
