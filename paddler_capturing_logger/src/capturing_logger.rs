use std::sync::Mutex;
use std::sync::PoisonError;

use log::LevelFilter;
use log::Log;
use log::Metadata;
use log::Record;
use log::set_logger;
use log::set_max_level;

use crate::captured_record::CapturedRecord;

pub struct CapturingLogger {
    records: Mutex<Vec<CapturedRecord>>,
}

impl CapturingLogger {
    /// # Panics
    ///
    /// Panics when another logger was installed first, because the records the test asserts on would never reach this one.
    #[must_use]
    pub fn install() -> &'static Self {
        let capturing_logger: &'static Self = Box::leak(Box::new(Self {
            records: Mutex::new(Vec::new()),
        }));

        assert!(
            set_logger(capturing_logger).is_ok(),
            "the capturing logger must be the only logger of the test process"
        );
        set_max_level(LevelFilter::Debug);

        capturing_logger
    }

    #[must_use]
    pub fn records_for(&self, target: &str) -> Vec<CapturedRecord> {
        self.records
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .iter()
            .filter(
                |CapturedRecord {
                     target: record_target,
                     ..
                 }| record_target == target,
            )
            .cloned()
            .collect()
    }
}

impl Log for CapturingLogger {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        self.records
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(CapturedRecord {
                level: record.level(),
                message: record.args().to_string(),
                target: record.target().to_owned(),
            });
    }

    fn flush(&self) {}
}

#[cfg(test)]
mod tests {
    use log::Level;
    use log::info;
    use log::trace;

    use super::CapturingLogger;
    use crate::captured_record::CapturedRecord;

    #[test]
    fn keeps_every_record_under_its_target() {
        let capturing_logger = CapturingLogger::install();

        info!("the record the test asserts on");

        assert_eq!(
            capturing_logger.records_for(module_path!()),
            vec![CapturedRecord {
                level: Level::Info,
                message: "the record the test asserts on".to_owned(),
                target: module_path!().to_owned(),
            }]
        );
    }

    #[test]
    fn leaves_out_the_trace_records_only_dependencies_emit() {
        let capturing_logger = CapturingLogger::install();

        trace!(target: "dependency_frame_dump", "a frame payload");

        assert_eq!(
            capturing_logger.records_for("dependency_frame_dump"),
            Vec::<CapturedRecord>::new()
        );
    }
}
