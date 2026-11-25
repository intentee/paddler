use log::Level;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapturedRecord {
    pub level: Level,
    pub message: String,
    pub target: String,
}
