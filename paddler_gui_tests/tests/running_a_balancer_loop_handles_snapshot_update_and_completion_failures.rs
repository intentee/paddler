use std::future::pending;
use std::time::Duration;

use anyhow::Result;
use anyhow::anyhow;
use iced::futures::StreamExt as _;
use iced::futures::channel::mpsc;
use paddler_gui::drive_balancer_loop_inner::drive_balancer_loop_inner;
use paddler_gui::message::Message;
use paddler_gui::running_balancer_snapshot::RunningBalancerSnapshot;
use paddler_types::balancer_desired_state::BalancerDesiredState;
use tokio::sync::broadcast;
use tokio::sync::watch;

const BALANCER_LOOP_TIMEOUT: Duration = Duration::from_secs(5);

#[tokio::test]
async fn snapshot_build_failure_exits_the_balancer_loop() -> Result<()> {
    let (_, pool_update_rx) = watch::channel(());
    let (_, holder_update_rx) = watch::channel(());
    let (_, desired_state_rx) = broadcast::channel::<BalancerDesiredState>(16);
    let (output, mut receiver) = mpsc::channel::<Message>(8);

    let driver = tokio::spawn(drive_balancer_loop_inner(
        BalancerDesiredState::default(),
        pool_update_rx,
        holder_update_rx,
        desired_state_rx,
        pending::<anyhow::Result<()>>(),
        |_| Err(anyhow!("snapshot build failed")),
        output,
    ));

    tokio::time::timeout(BALANCER_LOOP_TIMEOUT, driver).await??;

    assert!(receiver.next().await.is_none());

    Ok(())
}

#[tokio::test]
async fn pool_update_channel_closure_exits_the_balancer_loop_after_a_snapshot() -> Result<()> {
    let (_, holder_update_rx) = watch::channel(());
    let (_, desired_state_rx) = broadcast::channel::<BalancerDesiredState>(16);
    let (output, mut receiver) = mpsc::channel::<Message>(8);

    // Drop the pool sender so `changed()` will fail on the first await.
    let (_, pool_update_rx) = watch::channel(());

    let driver = tokio::spawn(drive_balancer_loop_inner(
        BalancerDesiredState::default(),
        pool_update_rx,
        holder_update_rx,
        desired_state_rx,
        pending::<anyhow::Result<()>>(),
        |_| Ok(RunningBalancerSnapshot::default()),
        output,
    ));

    let first_message = tokio::time::timeout(BALANCER_LOOP_TIMEOUT, receiver.next()).await?;
    assert!(matches!(first_message, Some(Message::RunningBalancer(_))));

    tokio::time::timeout(BALANCER_LOOP_TIMEOUT, driver).await??;

    Ok(())
}

#[tokio::test]
async fn desired_state_broadcast_closure_exits_the_balancer_loop_after_a_snapshot() -> Result<()> {
    let (pool_update_tx, pool_update_rx) = watch::channel(());
    let (holder_update_tx, holder_update_rx) = watch::channel(());
    let (desired_state_tx, desired_state_rx) = broadcast::channel::<BalancerDesiredState>(16);
    let (output, mut receiver) = mpsc::channel::<Message>(8);

    let driver = tokio::spawn(drive_balancer_loop_inner(
        BalancerDesiredState::default(),
        pool_update_rx,
        holder_update_rx,
        desired_state_rx,
        pending::<anyhow::Result<()>>(),
        |_| Ok(RunningBalancerSnapshot::default()),
        output,
    ));

    // Wait for the first snapshot to land, then close the desired-state broadcast.
    let first_message = tokio::time::timeout(BALANCER_LOOP_TIMEOUT, receiver.next()).await?;
    assert!(matches!(first_message, Some(Message::RunningBalancer(_))));

    drop(desired_state_tx);
    drop(pool_update_tx);
    drop(holder_update_tx);

    tokio::time::timeout(BALANCER_LOOP_TIMEOUT, driver).await??;

    Ok(())
}

#[tokio::test]
async fn completion_with_ok_emits_balancer_stopped_message() -> Result<()> {
    let (pool_update_tx, pool_update_rx) = watch::channel(());
    let (holder_update_tx, holder_update_rx) = watch::channel(());
    let (_desired_state_tx, desired_state_rx) = broadcast::channel::<BalancerDesiredState>(16);
    let (output, mut receiver) = mpsc::channel::<Message>(8);

    let driver = tokio::spawn(drive_balancer_loop_inner(
        BalancerDesiredState::default(),
        pool_update_rx,
        holder_update_rx,
        desired_state_rx,
        async { Ok(()) },
        |_| Ok(RunningBalancerSnapshot::default()),
        output,
    ));

    let mut observed_stopped = false;
    let collect = async {
        while let Some(message) = receiver.next().await {
            if matches!(message, Message::BalancerStopped) {
                observed_stopped = true;
                break;
            }
        }
    };

    tokio::time::timeout(BALANCER_LOOP_TIMEOUT, collect).await?;

    drop(pool_update_tx);
    drop(holder_update_tx);

    tokio::time::timeout(BALANCER_LOOP_TIMEOUT, driver).await??;

    assert!(observed_stopped);
    Ok(())
}

#[tokio::test]
async fn completion_with_error_emits_balancer_failed_message() -> Result<()> {
    let (pool_update_tx, pool_update_rx) = watch::channel(());
    let (holder_update_tx, holder_update_rx) = watch::channel(());
    let (_desired_state_tx, desired_state_rx) = broadcast::channel::<BalancerDesiredState>(16);
    let (output, mut receiver) = mpsc::channel::<Message>(8);

    let driver = tokio::spawn(drive_balancer_loop_inner(
        BalancerDesiredState::default(),
        pool_update_rx,
        holder_update_rx,
        desired_state_rx,
        async { Err(anyhow!("runner crashed")) },
        |_| Ok(RunningBalancerSnapshot::default()),
        output,
    ));

    let mut observed_failed = false;
    let collect = async {
        while let Some(message) = receiver.next().await {
            if matches!(message, Message::BalancerFailed(_)) {
                observed_failed = true;
                break;
            }
        }
    };

    tokio::time::timeout(BALANCER_LOOP_TIMEOUT, collect).await?;

    drop(pool_update_tx);
    drop(holder_update_tx);

    tokio::time::timeout(BALANCER_LOOP_TIMEOUT, driver).await??;

    assert!(observed_failed);
    Ok(())
}

#[tokio::test]
async fn snapshot_send_failure_after_first_iteration_exits_the_balancer_loop() -> Result<()> {
    let (pool_update_tx, pool_update_rx) = watch::channel(());
    let (holder_update_tx, holder_update_rx) = watch::channel(());
    let (desired_state_tx, desired_state_rx) = broadcast::channel::<BalancerDesiredState>(16);
    let (output, receiver) = mpsc::channel::<Message>(1);
    drop(receiver);

    let driver = tokio::spawn(drive_balancer_loop_inner(
        BalancerDesiredState::default(),
        pool_update_rx,
        holder_update_rx,
        desired_state_rx,
        pending::<anyhow::Result<()>>(),
        |_| Ok(RunningBalancerSnapshot::default()),
        output,
    ));

    tokio::time::timeout(BALANCER_LOOP_TIMEOUT, driver).await??;

    drop(pool_update_tx);
    drop(holder_update_tx);
    drop(desired_state_tx);

    Ok(())
}

#[tokio::test]
async fn desired_state_lagged_broadcast_continues_the_balancer_loop() -> Result<()> {
    let (pool_update_tx, pool_update_rx) = watch::channel(());
    let (holder_update_tx, holder_update_rx) = watch::channel(());
    let (desired_state_tx, desired_state_rx) = broadcast::channel::<BalancerDesiredState>(1);
    let (output, mut receiver) = mpsc::channel::<Message>(64);

    // Fill the broadcast buffer beyond capacity to trigger Lagged on first recv.
    for _ in 0..3 {
        desired_state_tx.send(BalancerDesiredState::default()).ok();
    }

    let driver = tokio::spawn(drive_balancer_loop_inner(
        BalancerDesiredState::default(),
        pool_update_rx,
        holder_update_rx,
        desired_state_rx,
        async { Ok(()) },
        |_| Ok(RunningBalancerSnapshot::default()),
        output,
    ));

    let mut observed_stopped = false;
    let collect = async {
        while let Some(message) = receiver.next().await {
            if matches!(message, Message::BalancerStopped) {
                observed_stopped = true;
                break;
            }
        }
    };

    tokio::time::timeout(BALANCER_LOOP_TIMEOUT, collect).await?;

    drop(pool_update_tx);
    drop(holder_update_tx);
    drop(desired_state_tx);

    tokio::time::timeout(BALANCER_LOOP_TIMEOUT, driver).await??;

    assert!(observed_stopped);
    Ok(())
}

#[tokio::test]
async fn desired_state_update_replaces_current_state_in_the_balancer_loop() -> Result<()> {
    let (pool_update_tx, pool_update_rx) = watch::channel(());
    let (holder_update_tx, holder_update_rx) = watch::channel(());
    let (desired_state_tx, desired_state_rx) = broadcast::channel::<BalancerDesiredState>(16);
    let (output, mut receiver) = mpsc::channel::<Message>(64);

    let driver = tokio::spawn(drive_balancer_loop_inner(
        BalancerDesiredState::default(),
        pool_update_rx,
        holder_update_rx,
        desired_state_rx,
        async { Ok(()) },
        |_| Ok(RunningBalancerSnapshot::default()),
        output,
    ));

    // Wait for the first snapshot, then publish a desired-state update.
    let first = tokio::time::timeout(BALANCER_LOOP_TIMEOUT, receiver.next()).await?;
    assert!(matches!(first, Some(Message::RunningBalancer(_))));

    desired_state_tx.send(BalancerDesiredState::default()).ok();

    let mut observed_stopped = false;
    let collect = async {
        while let Some(message) = receiver.next().await {
            if matches!(message, Message::BalancerStopped) {
                observed_stopped = true;
                break;
            }
        }
    };

    tokio::time::timeout(BALANCER_LOOP_TIMEOUT, collect).await?;

    drop(pool_update_tx);
    drop(holder_update_tx);
    drop(desired_state_tx);

    tokio::time::timeout(BALANCER_LOOP_TIMEOUT, driver).await??;

    assert!(observed_stopped);
    Ok(())
}
