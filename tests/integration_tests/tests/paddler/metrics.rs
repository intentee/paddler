use serde::Deserialize;

#[derive(Deserialize, Debug, Default, Clone, PartialEq)]
pub struct Metrics {
    pub paddler_slots_idle: u16,
    pub paddler_slots_processing: u16,
    pub paddler_requests_buffered: u16,
}

pub fn get_average(metrics: Vec<Metrics>) -> Metrics {
    if metrics.is_empty() {
        return Metrics::default();
    }
    let count = metrics.len() as f64;

    let mut paddler_slots_idle = 0u32;
    let mut paddler_slots_processing = 0u32;
    let mut paddler_requests_buffered = 0u32;

    for metric in metrics {
        paddler_slots_idle += metric.paddler_slots_idle as u32;
        paddler_slots_processing += metric.paddler_slots_processing as u32;
        paddler_requests_buffered += metric.paddler_requests_buffered as u32;
    }

    Metrics {
        paddler_slots_idle: ((paddler_slots_idle as f64) / count).round() as u16,
        paddler_slots_processing: ((paddler_slots_processing as f64) / count).round() as u16,
        paddler_requests_buffered: ((paddler_requests_buffered as f64) / count).round() as u16,
    }
}
