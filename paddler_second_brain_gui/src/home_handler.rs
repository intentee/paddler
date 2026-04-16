use crate::home_data::HomeData;

#[derive(Debug, Clone, Copy)]
pub enum Message {
    StartCluster,
    JoinCluster,
}

pub enum Action {
    StartCluster,
    JoinCluster,
}

impl HomeData {
    pub const fn update(message: Message) -> Action {
        match message {
            Message::StartCluster => Action::StartCluster,
            Message::JoinCluster => Action::JoinCluster,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Action;
    use super::HomeData;
    use super::Message;

    #[test]
    fn start_cluster_message_produces_start_cluster_action() {
        assert!(matches!(
            HomeData::update(Message::StartCluster),
            Action::StartCluster
        ));
    }

    #[test]
    fn join_cluster_message_produces_join_cluster_action() {
        assert!(matches!(
            HomeData::update(Message::JoinCluster),
            Action::JoinCluster
        ));
    }
}
