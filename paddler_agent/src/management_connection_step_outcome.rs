use std::mem::Discriminant;
use std::mem::discriminant;

use crate::management_connection_end::ManagementConnectionEnd;
use crate::management_connection_step::ManagementConnectionStep;

impl ManagementConnectionStep {
    pub fn outcome(&self) -> Result<Discriminant<Self>, &ManagementConnectionEnd> {
        match self {
            Self::End(management_connection_end) => Err(management_connection_end),
            continuing_step => Ok(discriminant(continuing_step)),
        }
    }
}
