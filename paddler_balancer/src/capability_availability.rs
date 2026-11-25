use crate::cluster_lacks_capability::ClusterLacksCapability;

#[derive(Debug, Eq, PartialEq)]
pub enum CapabilityAvailability {
    Available,
    Lacking(ClusterLacksCapability),
}
