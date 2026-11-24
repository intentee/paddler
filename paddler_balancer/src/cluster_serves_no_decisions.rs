use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ClusterServesNoDecisions;

impl Display for ClusterServesNoDecisions {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(
            "No agent serves or is loading a decision model, so the cluster serves no decisions",
        )
    }
}
