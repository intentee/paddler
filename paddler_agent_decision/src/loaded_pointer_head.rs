use std::path::PathBuf;

use paddler_agent_pointer_head::pointer_head::PointerHead;

pub struct LoadedPointerHead {
    pub pointer_head: PointerHead,
    pub pointer_head_path: PathBuf,
}
