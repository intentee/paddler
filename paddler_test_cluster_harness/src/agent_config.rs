#[derive(Clone, Debug)]
pub struct AgentConfig {
    pub name: String,
    pub slot_count: i32,
}

impl AgentConfig {
    #[must_use]
    pub fn single(slot_count: i32) -> Self {
        Self {
            name: "test-agent".to_owned(),
            slot_count,
        }
    }

    #[must_use]
    pub fn uniform(count: usize, slot_count: i32) -> Vec<Self> {
        (0..count)
            .map(|agent_index| Self {
                name: format!("test-agent-{agent_index}"),
                slot_count,
            })
            .collect()
    }
}
