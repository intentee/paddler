#[derive(Clone, Copy)]
pub enum ResponsesOutputItemKind {
    FunctionCall,
    Message,
    Reasoning,
}

impl ResponsesOutputItemKind {
    #[must_use]
    pub fn item_id(self, output_index: usize) -> String {
        let prefix = match self {
            Self::FunctionCall => "fc",
            Self::Message => "msg",
            Self::Reasoning => "rs",
        };

        format!("{prefix}_{output_index}")
    }
}

#[cfg(test)]
mod tests {
    use super::ResponsesOutputItemKind;

    #[test]
    fn names_each_output_item_after_its_kind_and_position() {
        assert_eq!(
            [
                ResponsesOutputItemKind::Reasoning,
                ResponsesOutputItemKind::Message,
                ResponsesOutputItemKind::FunctionCall,
            ]
            .map(|item_kind| item_kind.item_id(2)),
            ["rs_2", "msg_2", "fc_2"]
        );
    }
}
