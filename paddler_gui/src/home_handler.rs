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
