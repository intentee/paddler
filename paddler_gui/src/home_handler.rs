use crate::home_data::HomeData;

#[derive(Debug, Clone, Copy)]
pub enum Message {
    StartBalancer,
    JoinBalancer,
}

pub enum Action {
    StartBalancer,
    JoinBalancer,
}

impl HomeData {
    pub const fn update(message: Message) -> Action {
        match message {
            Message::StartBalancer => Action::StartBalancer,
            Message::JoinBalancer => Action::JoinBalancer,
        }
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use anyhow::bail;

    use super::Action;
    use super::HomeData;
    use super::Message;

    #[test]
    fn start_balancer_message_dispatches_to_start_balancer_action() -> Result<()> {
        match HomeData::update(Message::StartBalancer) {
            Action::StartBalancer => Ok(()),
            Action::JoinBalancer => bail!("expected StartBalancer action"),
        }
    }

    #[test]
    fn join_balancer_message_dispatches_to_join_balancer_action() -> Result<()> {
        match HomeData::update(Message::JoinBalancer) {
            Action::JoinBalancer => Ok(()),
            Action::StartBalancer => bail!("expected JoinBalancer action"),
        }
    }
}
