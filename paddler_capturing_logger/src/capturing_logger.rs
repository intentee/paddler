use std::sync::Mutex;
use std::sync::Once;
use std::sync::PoisonError;

use log::LevelFilter;
use log::Log;
use log::Metadata;
use log::Record;
use log::set_logger;
use log::set_max_level;

use crate::captured_record::CapturedRecord;

static CAPTURING_LOGGER: CapturingLogger = CapturingLogger {
    records: Mutex::new(Vec::new()),
};

static INSTALLATION: Once = Once::new();

pub struct CapturingLogger {
    records: Mutex<Vec<CapturedRecord>>,
}

impl CapturingLogger {
    /// # Panics
    ///
    /// Panics when another logger was installed first, because the records the test asserts on would never reach this one.
    pub fn install() {
        INSTALLATION.call_once(|| {
            assert!(
                set_logger(&CAPTURING_LOGGER).is_ok(),
                "the capturing logger must be the only logger of the test process"
            );
            set_max_level(LevelFilter::Trace);
        });
    }

    #[must_use]
    pub fn records_for(target: &str) -> Vec<CapturedRecord> {
        CAPTURING_LOGGER
            .records
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

    use super::CapturingLogger;
    use crate::captured_record::CapturedRecord;

    #[test]
    fn keeps_every_record_under_its_target() {
        CapturingLogger::install();

        info!("the record the test asserts on");

        assert_eq!(
            CapturingLogger::records_for(module_path!()),
            vec![CapturedRecord {
                level: Level::Info,
                message: "the record the test asserts on".to_owned(),
                target: module_path!().to_owned(),
            }]
        );
    }
}
