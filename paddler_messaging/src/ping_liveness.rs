use crate::constants::UNANSWERED_PINGS_TOLERATED;
use crate::ping_turn::PingTurn;

const PINGS_ALLOWED_WITHOUT_A_PONG: u32 = UNANSWERED_PINGS_TOLERATED + 1;

pub struct PingLiveness {
    pings_left_without_a_pong: u32,
}

impl PingLiveness {
    #[must_use]
    pub const fn next_ping(&mut self) -> PingTurn {
        match self.pings_left_without_a_pong.checked_sub(1) {
            Some(pings_left_without_a_pong) => {
                self.pings_left_without_a_pong = pings_left_without_a_pong;

                PingTurn::SendPing
            }
            None => PingTurn::PeerStoppedAnswering,
        }
    }

    pub const fn record_pong(&mut self) {
        self.pings_left_without_a_pong = PINGS_ALLOWED_WITHOUT_A_PONG;
    }
}

impl Default for PingLiveness {
    fn default() -> Self {
        Self {
            pings_left_without_a_pong: PINGS_ALLOWED_WITHOUT_A_PONG,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::iter::repeat_with;

    use super::PingLiveness;
    use crate::constants::UNANSWERED_PINGS_TOLERATED;
    use crate::ping_turn::PingTurn;

    fn pings_until_the_peer_is_unresponsive(ping_liveness: &mut PingLiveness) -> u32 {
        let sent_pings = repeat_with(|| ping_liveness.next_ping())
            .take_while(|ping_turn| *ping_turn == PingTurn::SendPing)
            .count();

        u32::try_from(sent_pings).expect("the sent pings must fit in u32")
    }

    #[test]
    fn a_peer_is_unresponsive_once_more_pings_than_tolerated_go_unanswered() {
        assert_eq!(
            pings_until_the_peer_is_unresponsive(&mut PingLiveness::default()),
            UNANSWERED_PINGS_TOLERATED + 1
        );
    }

    #[test]
    fn a_pong_forgives_the_pings_sent_before_it() {
        let mut ping_liveness = PingLiveness::default();

        assert_eq!(ping_liveness.next_ping(), PingTurn::SendPing);
        ping_liveness.record_pong();

        assert_eq!(
            pings_until_the_peer_is_unresponsive(&mut ping_liveness),
            UNANSWERED_PINGS_TOLERATED + 1
        );
    }
}
