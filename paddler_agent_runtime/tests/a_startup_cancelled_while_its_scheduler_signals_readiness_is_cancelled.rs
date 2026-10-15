use std::future::Future as _;
use std::future::poll_fn;
use std::mem::discriminant;
use std::pin::pin;
use std::sync::mpsc::channel;
use std::task::Poll;
use std::thread::spawn;

use tokio::sync::mpsc::unbounded_channel;
use tokio_util::sync::CancellationToken;

use paddler_agent_runtime::await_scheduler_startup::await_scheduler_startup;
use paddler_agent_runtime::scheduler_ready::SchedulerReady;
use paddler_agent_runtime::send_startup_signal::send_startup_signal;
use paddler_agent_runtime::startup_signal_delivery::StartupSignalDelivery;
use paddler_messaging::model_metadata::ModelMetadata;

use crate::number_request_preparer::NumberRequestPreparer;
use crate::reports_ready_model_metadata::ReportsReadyModelMetadata as _;

#[tokio::test]
async fn a_startup_cancelled_while_its_scheduler_signals_readiness_is_cancelled() {
    let NumberRequestPreparer {
        request_preparer, ..
    } = NumberRequestPreparer::new();
    let (scheduler_startup_tx, scheduler_startup_rx) = unbounded_channel();
    let (readiness_requested_tx, readiness_requested_rx) = channel();
    let (readiness_signalled_tx, readiness_signalled_rx) = channel();
    let scheduler_thread_handle = spawn(move || {
        readiness_requested_rx
            .recv()
            .expect("the test must request the readiness signal");
        readiness_signalled_tx
            .send(send_startup_signal(
                &scheduler_startup_tx,
                Ok::<_, String>(SchedulerReady {
                    model_metadata: ModelMetadata::default(),
                    request_preparer,
                }),
            ))
            .expect("the test must wait for the readiness signal");
    });
    let cancellation_token = CancellationToken::new();
    let mut startup = pin!(await_scheduler_startup(
        &cancellation_token,
        scheduler_startup_rx,
        scheduler_thread_handle,
    ));

    assert!(
        poll_fn(|context| Poll::Ready(startup.as_mut().poll(context).is_pending())).await,
        "the startup must wait for its scheduler"
    );

    readiness_requested_tx
        .send(())
        .expect("the scheduler thread must wait for the readiness request");

    assert_eq!(
        discriminant(
            &readiness_signalled_rx
                .recv()
                .expect("the scheduler thread must report its readiness signal")
        ),
        discriminant(&StartupSignalDelivery::Delivered)
    );

    cancellation_token.cancel();

    assert_eq!(
        startup
            .await
            .expect("a cancelled startup must join its scheduler thread")
            .ready_model_metadata(),
        None
    );
}
