use crate::cluster_serves_no_decisions::ClusterServesNoDecisions;

#[derive(Debug, Eq, PartialEq)]
pub enum DecisionsServing {
    NotServed(ClusterServesNoDecisions),
    Served,
}
