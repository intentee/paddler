use core::panic;

use anyhow::Result;
use cucumber::then;
use openmetrics_parser::MetricFamily;
use openmetrics_parser::MetricsExposition;
use openmetrics_parser::PrometheusType;
use openmetrics_parser::PrometheusValue;
use openmetrics_parser::prometheus::parse_prometheus;
use reqwest::Response;

use crate::metrics::Metrics;
use crate::paddler_world::PaddlerWorld;

async fn fetch_metrics(management_addr: u16) -> Result<Response> {
    let response =
        reqwest::get(format!("http://localhost:{management_addr}/api/v1/metrics")).await?;

    if !response.status().is_success() {
        return Err(anyhow::anyhow!(
            "Failed to fetch metrics: Expected status 200, got {}",
            response.status()
        ));
    }

    Ok(response)
}

fn extract_gauge_u16(metrics: &MetricFamily<PrometheusType, PrometheusValue>) -> Option<u16> {
    for metrics in metrics.iter_samples() {
        match metrics.value {
            PrometheusValue::Unknown(_) => return None,
            PrometheusValue::Gauge(value) => {
                return Some(value.as_i64().expect("Failed to parse gauge value to i64") as u16);
            }
            PrometheusValue::Counter(_) => return None,
            PrometheusValue::Histogram(_) => return None,
            PrometheusValue::Summary(_) => return None,
        }
    }

    return None;
}

fn get_sample_values(metric_family: MetricsExposition<PrometheusType, PrometheusValue>) -> Metrics {
    let mut metrics = Metrics::default();

    if let Some(slots_idle) = metric_family.families.get("paddler_slots_idle") {
        if let Some(slots_idle) = extract_gauge_u16(slots_idle) {
            metrics.paddler_slots_idle = slots_idle;
        }
    }

    if let Some(slots_processing) = metric_family.families.get("paddler_slots_processing") {
        if let Some(slots_processing) = extract_gauge_u16(slots_processing) {
            metrics.paddler_slots_processing = slots_processing;
        }
    }

    if let Some(requests_buffered) = metric_family.families.get("paddler_requests_buffered") {
        if let Some(requests_buffered) = extract_gauge_u16(requests_buffered) {
            metrics.paddler_requests_buffered = requests_buffered;
        }
    }

    metrics
}

fn parse_metrics(body_metrics: &str) -> Result<Metrics> {
    match parse_prometheus(body_metrics) {
        Ok(metric_family) => {
            let metric_values = get_sample_values(metric_family);

            return Ok(metric_values);
        }
        Err(e) => {
            return Err(anyhow::anyhow!(
                "Failed to fetch metrics: Expected status 200, got {e}"
            ));
        }
    };
}

#[then(expr = "exported metrics are stored")]
pub async fn then_exported_metrics_are_stored(world: &mut PaddlerWorld) -> Result<()> {
    let metrics = fetch_metrics(8095).await?.text().await?;

    let metrics = parse_metrics(&metrics).expect("Failed to parse metrics");

    world.statsd.metrics.push(metrics);

    Ok(())
}
