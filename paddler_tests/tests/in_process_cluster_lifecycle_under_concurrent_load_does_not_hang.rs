use std::time::Duration;
use std::time::Instant;

use anyhow::Context as _;
use anyhow::Result;
use paddler_tests::in_process_cluster_params::InProcessClusterParams;
use paddler_tests::start_in_process_cluster::start_in_process_cluster;
use tokio::task::JoinSet;
use tokio::time::timeout;

const CONCURRENT_LIFECYCLES: usize = 8;
const LIFECYCLES_PER_TASK: usize = 5;

/// Eight concurrent tokio tasks × five lifecycles each = forty total lifecycles
/// overlapping in time, approximating the contention pattern of the full
/// integration suite where multiple test binaries spawn clusters in parallel.
/// Total budget is generous enough to absorb scheduling jitter but tight enough
/// to catch a stuck shutdown — a single hung lifecycle of 60 s+ would blow
/// past this bound.
const TOTAL_BUDGET: Duration = Duration::from_secs(60);

#[tokio::test(flavor = "multi_thread")]
async fn in_process_cluster_lifecycle_under_concurrent_load_does_not_hang() -> Result<()> {
    let started_at = Instant::now();

    let mut join_set: JoinSet<Result<()>> = JoinSet::new();

    for _ in 0..CONCURRENT_LIFECYCLES {
        join_set.spawn(async move {
            for _ in 0..LIFECYCLES_PER_TASK {
                let cluster = start_in_process_cluster(InProcessClusterParams {
                    wait_for_slots_ready: false,
                    ..InProcessClusterParams::default()
                })
                .await?;
                cluster.shutdown().await?;
            }
            Ok(())
        });
    }

    timeout(TOTAL_BUDGET, async {
        while let Some(join_result) = join_set.join_next().await {
            join_result
                .context("concurrent-lifecycle task panicked")?
                .context("concurrent-lifecycle task returned an error")?;
        }
        Ok::<_, anyhow::Error>(())
    })
    .await
    .with_context(|| {
        format!(
            "concurrent in-process cluster lifecycles did not all complete within {TOTAL_BUDGET:?}; \
             this is the same symptom the user observed in the full integration suite: shutdown \
             stalls under cross-test contention"
        )
    })??;

    let elapsed = started_at.elapsed();

    assert!(
        elapsed <= TOTAL_BUDGET,
        "concurrent lifecycles took {elapsed:?}, over budget of {TOTAL_BUDGET:?}"
    );

    Ok(())
}
