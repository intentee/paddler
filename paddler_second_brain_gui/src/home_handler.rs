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
