#![cfg(feature = "tests_that_use_llms")]

use std::future::Future as _;
use std::pin::pin;
use std::task::Context;
use std::task::Waker;

use tokio_util::sync::CancellationToken;

use paddler_agent_runtime::shuts_down::ShutsDown as _;
use paddler_blocking_thread_fixture::occupied_blocking_thread::OccupiedBlockingThread;
use paddler_blocking_thread_fixture::single_blocking_thread_runtime::single_blocking_thread_runtime;

use crate::kev_model_serving_without_decisions::KevModelServingWithoutDecisions;

#[test]
fn shutting_down_a_served_model_closes_every_capability_at_once() {
    single_blocking_thread_runtime()
        .expect("the test runtime must build")
        .block_on(async {
            let KevModelServingWithoutDecisions { served_model, .. } =
                KevModelServingWithoutDecisions::start(&CancellationToken::new()).await;
            let embeddings_preparations = served_model
                .capabilities
                .embeddings
                .request_preparer
                .preparation_tasks
                .clone();
            let text_generation_preparations = served_model
                .capabilities
                .text_generation
                .request_preparer
                .preparation_tasks
                .clone();
            let occupied_blocking_thread = OccupiedBlockingThread::occupy();
            let mut shutting_down = pin!(served_model.shut_down());

            assert!(
                shutting_down
                    .as_mut()
                    .poll(&mut Context::from_waker(Waker::noop()))
                    .is_pending()
            );
            assert!(embeddings_preparations.is_closed());
            assert!(text_generation_preparations.is_closed());

            occupied_blocking_thread
                .release()
                .await
                .expect("the blocking thread occupant must be released");
            shutting_down.await;
        });
}
