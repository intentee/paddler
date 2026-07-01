use core::panic;

use anyhow::Result;
use cucumber::gherkin::Step;
use cucumber::then;

use crate::metrics::Metrics;
use crate::metrics::get_average;
use crate::paddler_world::PaddlerWorld;

#[then(expr = "average metrics are:")]
pub async fn then_metrics_report(world: &mut PaddlerWorld, step: &Step) -> Result<()> {
    let all_metrics = match world.statsd.metrics.len() {
        0 => vec![Metrics::default(), Metrics::default()],
        _ => world.statsd.metrics.clone(),
    };

    let average_metrics = get_average(all_metrics.clone());

    let mut table_metrics = Metrics::default();

    if let Some(table) = step.table.as_ref() {
        for row in &table.rows {
            match row[0].as_str() {
                "slots_idle" => {
                    table_metrics.paddler_slots_idle = row[1]
                        .parse()
                        .expect("Failed to parse 'slots_idle' as a valid number")
                }
                "slots_processing" => {
                    table_metrics.paddler_slots_processing = row[1]
                        .parse()
                        .expect("Failed to parse 'slots_processing' as a valid number")
                }
                "requests_buffered" => {
                    table_metrics.paddler_requests_buffered = row[1]
                        .parse()
                        .expect("Failed to parse 'requests_buffered' as a valid number")
                }
                _ => (),
            }
        }
    };

    assert_eq!(table_metrics, average_metrics);

    Ok(())
}
