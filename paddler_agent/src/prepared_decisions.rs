use paddler_agent_decision::loaded_pointer_head::LoadedPointerHead;

pub enum PreparedDecisions {
    Disabled,
    Enabled(Box<LoadedPointerHead>),
}
