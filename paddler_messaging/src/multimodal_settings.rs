use std::num::NonZeroU32;

use serde::Deserialize;
use serde::Serialize;

use crate::agent_desired_model::AgentDesiredModel;

const DEFAULT_IMAGE_RESIZE_TO_FIT: NonZeroU32 = NonZeroU32::new(1024).unwrap();

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MultimodalSettings {
    pub image_resize_to_fit: NonZeroU32,
    pub projection: AgentDesiredModel,
}

impl Default for MultimodalSettings {
    fn default() -> Self {
        Self {
            image_resize_to_fit: DEFAULT_IMAGE_RESIZE_TO_FIT,
            projection: AgentDesiredModel::None,
        }
    }
}
