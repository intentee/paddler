use std::sync::Arc;
use std::time::Duration;

use tokio::sync::watch;
use tokio::time::error::Elapsed;
use tokio::time::timeout;

use paddler_messaging::buffered_request_manager_snapshot::BufferedRequestManagerSnapshot;
use paddler_messaging::inference_mode::InferenceMode;
use paddler_messaging::produces_snapshot::ProducesSnapshot;
use paddler_messaging::subscribes_to_updates::SubscribesToUpdates;

use crate::agent_controller_pool::AgentControllerPool;
use crate::buffered_request_agent_wait_result::BufferedRequestAgentWaitResult;
use crate::buffered_request_counter::BufferedRequestCounter;
use crate::dispatched_agent::DispatchedAgent;

pub struct BufferedRequestManager {
    agent_controller_pool: Arc<AgentControllerPool>,
    pub buffered_request_counter: Arc<BufferedRequestCounter>,
    buffered_request_timeout: Duration,
    update_tx: watch::Sender<()>,
}

impl BufferedRequestManager {
    #[must_use]
    pub fn new(
        agent_controller_pool: Arc<AgentControllerPool>,
        buffered_request_timeout: Duration,
        max_buffered_requests: u64,
    ) -> Self {
        let (update_tx, _initial_rx) = watch::channel(());

        Self {
            agent_controller_pool,
            buffered_request_counter: Arc::new(BufferedRequestCounter::new(
                update_tx.clone(),
                max_buffered_requests,
            )),
            buffered_request_timeout,
            update_tx,
        }
    }

    #[must_use]
    pub fn take_available_agent(&self, inference_mode: InferenceMode) -> Option<DispatchedAgent> {
        self.agent_controller_pool
            .take_least_busy_agent_controller(inference_mode)
    }

    pub async fn wait_for_available_agent(
        &self,
        inference_mode: InferenceMode,
    ) -> BufferedRequestAgentWaitResult {
        if let Some(dispatched_agent) = self.take_available_agent(inference_mode) {
            return BufferedRequestAgentWaitResult::Found(dispatched_agent);
        }

        let Some(_buffered_request_count_guard) = self.buffered_request_counter.try_admit() else {
            return BufferedRequestAgentWaitResult::BufferOverflow;
        };

        match timeout(
            self.buffered_request_timeout,
            self.agent_controller_pool
                .next_available_agent(inference_mode),
        )
        .await
        {
            Ok(dispatched_agent) => BufferedRequestAgentWaitResult::Found(dispatched_agent),
            Err(Elapsed { .. }) => BufferedRequestAgentWaitResult::Timeout,
        }
    }
}

impl ProducesSnapshot for BufferedRequestManager {
    type Snapshot = BufferedRequestManagerSnapshot;

    fn make_snapshot(&self) -> Self::Snapshot {
        BufferedRequestManagerSnapshot {
            buffered_requests_current: self.buffered_request_counter.get(),
        }
    }
}

impl SubscribesToUpdates for BufferedRequestManager {
    fn subscribe_to_updates(&self) -> watch::Receiver<()> {
        self.update_tx.subscribe()
    }
}

#[cfg(test)]
mod tests {
    use std::hint::spin_loop;
    use std::mem::discriminant;
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;
    use std::thread::available_parallelism;
    use std::thread::scope;
    use std::time::Duration;

    use tokio::runtime::Builder;
    use tokio::select;
    use tokio::sync::mpsc;
    use tokio_util::sync::CancellationToken;

    use paddler_messaging::atomic_value::AtomicValue;
    use paddler_messaging::inference_mode::InferenceMode;
    use paddler_messaging::subscribes_to_updates::SubscribesToUpdates as _;

    use super::BufferedRequestManager;
    use crate::agent_controller_pool::AgentControllerPool;
    use crate::buffered_request_agent_wait_result::BufferedRequestAgentWaitResult;

    const BUFFER_CAPACITY: u64 = 1;
    const RACE_ROUNDS: usize = 64;

    struct RaceOutcome {
        admitted_callers: u64,
        overflowed_callers: usize,
    }

    fn race_callers_for_one_buffered_place(racing_callers: usize) -> RaceOutcome {
        let buffered_request_manager = BufferedRequestManager::new(
            Arc::new(AgentControllerPool::default()),
            Duration::MAX,
            BUFFER_CAPACITY,
        );
        let start_line = AtomicValue::<AtomicBool>::new(false);
        let admitted_callers_release = CancellationToken::new();
        let (overflow_tx, mut overflow_rx) = mpsc::unbounded_channel();

        scope(|caller_scope| {
            let caller_handles: Vec<_> = (0..racing_callers)
                .map(|_caller_index| {
                    let admitted_callers_release = &admitted_callers_release;
                    let buffered_request_manager = &buffered_request_manager;
                    let overflow_tx = overflow_tx.clone();
                    let start_line = &start_line;

                    caller_scope.spawn(move || {
                        let caller_runtime = Builder::new_current_thread()
                            .enable_time()
                            .build()
                            .expect("a racing caller must build its runtime");

                        while !start_line.get() {
                            spin_loop();
                        }

                        caller_runtime.block_on(async {
                            if let Some(wait_result) = admitted_callers_release
                                .run_until_cancelled(
                                    buffered_request_manager
                                        .wait_for_available_agent(InferenceMode::TextGeneration),
                                )
                                .await
                                && discriminant(&wait_result)
                                    == discriminant(&BufferedRequestAgentWaitResult::BufferOverflow)
                            {
                                overflow_tx
                                    .send(())
                                    .expect("the observer must still count overflows");
                            }
                        });
                    })
                })
                .collect();

            let mut buffered_request_updates = buffered_request_manager.subscribe_to_updates();
            let observer_runtime = Builder::new_current_thread()
                .build()
                .expect("the observer must build its runtime");

            start_line.set(true);

            let overflowed_callers = observer_runtime.block_on(async {
                let mut overflowed_callers = 0;

                while overflowed_callers
                    + usize::try_from(buffered_request_manager.buffered_request_counter.get())
                        .expect("the buffered request count must not be negative")
                    < racing_callers
                {
                    select! {
                        Some(()) = overflow_rx.recv() => overflowed_callers += 1,
                        buffered_request_update = buffered_request_updates.changed() => {
                            buffered_request_update
                                .expect("the buffered request manager must keep announcing updates");
                        }
                    }
                }

                overflowed_callers
            });
            let admitted_callers = buffered_request_manager.buffered_request_counter.get();

            admitted_callers_release.cancel();

            for caller_handle in caller_handles {
                caller_handle
                    .join()
                    .expect("a racing caller must not panic");
            }

            RaceOutcome {
                admitted_callers,
                overflowed_callers,
            }
        })
    }

    #[test]
    fn admits_no_more_concurrent_requests_than_the_buffer_holds() {
        let racing_callers = available_parallelism()
            .expect("the test must know how many callers can race")
            .get();

        for _race_round in 0..RACE_ROUNDS {
            let RaceOutcome {
                admitted_callers,
                overflowed_callers,
            } = race_callers_for_one_buffered_place(racing_callers);

            assert_eq!(admitted_callers, BUFFER_CAPACITY);
            assert_eq!(overflowed_callers, racing_callers - 1);
        }
    }
}
