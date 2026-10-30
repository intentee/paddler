use crate::ping_turn::PingTurn;
use crate::unanswered_pings_tolerated::UNANSWERED_PINGS_TOLERATED;

#[derive(Default)]
pub struct PingLiveness {
    unanswered_pings: u32,
}

impl PingLiveness {
    pub const fn record_pong(&mut self) {
        self.unanswered_pings = 0;
    }

    #[must_use]
    pub const fn next_ping(&mut self) -> PingTurn {
        if self.unanswered_pings > UNANSWERED_PINGS_TOLERATED {
            PingTurn::PeerStoppedAnswering
        } else {
            self.unanswered_pings += 1;

            PingTurn::SendPing
        }
    }
}

#[cfg(test)]
mod tests {
    use super::PingLiveness;
    use crate::ping_turn::PingTurn;
    use crate::unanswered_pings_tolerated::UNANSWERED_PINGS_TOLERATED;

    fn pings_until_the_peer_is_unresponsive(ping_liveness: &mut PingLiveness) -> u32 {
        let mut sent_pings = 0;

        while ping_liveness.next_ping() == PingTurn::SendPing {
            sent_pings += 1;
        }

        sent_pings
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
