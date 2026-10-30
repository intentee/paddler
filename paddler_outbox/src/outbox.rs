use std::collections::VecDeque;

use parking_lot::Mutex;
use tokio::sync::Notify;

pub struct Outbox<TMessage> {
    message_arrived: Notify,
    messages: Mutex<VecDeque<TMessage>>,
}

impl<TMessage> Outbox<TMessage> {
    pub fn push(&self, message: TMessage) {
        self.messages.lock().push_back(message);
        self.message_arrived.notify_one();
    }

    pub async fn next(&self) -> TMessage {
        loop {
            let next_message = self.messages.lock().pop_front();

            if let Some(message) = next_message {
                return message;
            }

            self.message_arrived.notified().await;
        }
    }
}

impl<TMessage> Default for Outbox<TMessage> {
    fn default() -> Self {
        Self {
            message_arrived: Notify::new(),
            messages: Mutex::new(VecDeque::new()),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use tokio::spawn;
    use tokio::task::yield_now;

    use super::Outbox;

    #[tokio::test]
    async fn delivers_messages_in_the_order_they_were_pushed() {
        let outbox = Outbox::default();

        outbox.push(1);
        outbox.push(2);

        assert_eq!([outbox.next().await, outbox.next().await], [1, 2]);
    }

    #[tokio::test]
    async fn wakes_a_consumer_waiting_for_a_message_pushed_later() {
        let outbox = Arc::new(Outbox::default());
        let waiting_consumer = spawn({
            let outbox = outbox.clone();

            async move { outbox.next().await }
        });

        yield_now().await;
        outbox.push(7);

        assert_eq!(waiting_consumer.await.unwrap(), 7);
    }
}
