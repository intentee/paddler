#[derive(Debug)]
pub struct OpenCodeRunOutcome {
    pub exit_success: bool,
    pub stderr: String,
    pub stdout: String,
}
