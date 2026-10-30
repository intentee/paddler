use nanoid::nanoid;

#[derive(Clone, Copy)]
pub enum ResponsesOutputItemKind {
    FunctionCall,
    Message,
    Reasoning,
}

impl ResponsesOutputItemKind {
    #[must_use]
    pub fn item_id(self) -> String {
        let prefix = match self {
            Self::FunctionCall => "fc",
            Self::Message => "msg",
            Self::Reasoning => "rs",
        };

        format!("{prefix}_{}", nanoid!())
    }
}

#[cfg(test)]
mod tests {
    use super::ResponsesOutputItemKind;

    #[test]
    fn names_each_output_item_after_its_kind() {
        assert!(
            ResponsesOutputItemKind::Reasoning
                .item_id()
                .starts_with("rs_")
        );
        assert!(
            ResponsesOutputItemKind::Message
                .item_id()
                .starts_with("msg_")
        );
        assert!(
            ResponsesOutputItemKind::FunctionCall
                .item_id()
                .starts_with("fc_")
        );
    }
}
