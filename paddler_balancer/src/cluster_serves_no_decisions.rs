use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ClusterServesNoDecisions;

impl Display for ClusterServesNoDecisions {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("No pointer head is configured, so the cluster serves no decisions")
    }
}
