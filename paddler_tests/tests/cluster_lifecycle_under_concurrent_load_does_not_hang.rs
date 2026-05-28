use std::time::Duration;
use std::time::Instant;

use anyhow::Context as _;
use anyhow::Result;
use paddler_tests::cluster_params::ClusterParams;
use paddler_tests::start_cluster::start_cluster;
use tokio::task::JoinSet;
use tokio::time::timeout;

const CONCURRENT_LIFECYCLES: usize = 8;
const LIFECYCLES_PER_TASK: usize = 5;

const TOTAL_BUDGET: Duration = Duration::from_mins(1);

#[tokio::test(flavor = "multi_thread")]
async fn cluster_lifecycle_under_concurrent_load_does_not_hang() -> Result<()> {
    let started_at = Instant::now();

    let mut join_set: JoinSet<Result<()>> = JoinSet::new();

    for _ in 0..CONCURRENT_LIFECYCLES {
        join_set.spawn(async move {
            for _ in 0..LIFECYCLES_PER_TASK {
                let cluster = start_cluster(ClusterParams {
                    wait_for_slots_ready: false,
                    ..ClusterParams::default()
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
