use paddler_agent_pointer_head::pointer_head::PointerHead;

pub struct DecisionSchedulerContext {
    pub agent_name: Option<String>,
    pub n_batch: usize,
    pub pointer_head: PointerHead,
}
