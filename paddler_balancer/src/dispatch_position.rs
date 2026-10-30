#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DispatchPosition {
    Arriving,
    Waiting { ticket: u64 },
}

impl DispatchPosition {
    #[must_use]
    pub const fn waits_behind(&self, other_ticket: u64) -> bool {
        match self {
            Self::Arriving => true,
            Self::Waiting { ticket } => other_ticket < *ticket,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::DispatchPosition;

    #[test]
    fn an_arriving_request_waits_behind_every_waiting_request() {
        assert!(DispatchPosition::Arriving.waits_behind(u64::MAX));
    }

    #[test]
    fn a_waiting_request_waits_behind_only_the_requests_that_joined_before_it() {
        let dispatch_position = DispatchPosition::Waiting { ticket: 3 };

        assert_eq!(
            [2, 3, 4].map(|other_ticket| dispatch_position.waits_behind(other_ticket)),
            [true, false, false]
        );
    }
}
