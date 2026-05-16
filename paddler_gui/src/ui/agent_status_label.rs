use paddler_types::agent_controller_snapshot::AgentControllerSnapshot;
use paddler_types::agent_state_application_status::AgentStateApplicationStatus;

pub fn agent_status_label(snapshot: &AgentControllerSnapshot) -> String {
    let is_downloading =
        snapshot.download_total > 0 && snapshot.download_current < snapshot.download_total;

    if is_downloading {
        #[expect(
            clippy::cast_precision_loss,
            reason = "download sizes fit in f32 mantissa"
        )]
        let percentage =
            (snapshot.download_current as f32 / snapshot.download_total as f32) * 100.0;

        return format!("Downloading ({percentage:.0}%)");
    }

    if snapshot.model_path.is_none() {
        return "Waiting for model...".to_owned();
    }

    match snapshot.state_application_status {
        AgentStateApplicationStatus::Applied => "OK".to_owned(),
        AgentStateApplicationStatus::Fresh => "Pending".to_owned(),
        AgentStateApplicationStatus::AttemptedAndRetrying => "Retrying".to_owned(),
        AgentStateApplicationStatus::Stuck => "Retrying, but seems stuck?".to_owned(),
        AgentStateApplicationStatus::AttemptedAndNotAppliable => "Needs your help".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use anyhow::Result;
    use paddler_types::agent_controller_snapshot::AgentControllerSnapshot;
    use paddler_types::agent_state_application_status::AgentStateApplicationStatus;

    use super::agent_status_label;

    fn snapshot_with(
        download_current: usize,
        download_total: usize,
        model_path: Option<&str>,
        status: AgentStateApplicationStatus,
    ) -> AgentControllerSnapshot {
        AgentControllerSnapshot {
            desired_slots_total: 0,
            download_current,
            download_filename: None,
            download_total,
            id: "snapshot-id".to_owned(),
            issues: BTreeSet::new(),
            model_path: model_path.map(str::to_owned),
            name: None,
            slots_processing: 0,
            slots_total: 0,
            state_application_status: status,
            uses_chat_template_override: false,
        }
    }

    #[test]
    fn label_reports_download_progress_percentage_when_a_download_is_in_progress() -> Result<()> {
        let snapshot = snapshot_with(25, 100, None, AgentStateApplicationStatus::Fresh);

        assert_eq!(agent_status_label(&snapshot), "Downloading (25%)");
        Ok(())
    }

    #[test]
    fn label_says_waiting_for_model_when_no_model_is_loaded_and_no_download_is_active() -> Result<()>
    {
        let snapshot = snapshot_with(0, 0, None, AgentStateApplicationStatus::Fresh);

        assert_eq!(agent_status_label(&snapshot), "Waiting for model...");
        Ok(())
    }

    #[test]
    fn label_says_ok_when_state_is_applied() -> Result<()> {
        let snapshot = snapshot_with(
            0,
            0,
            Some("/models/model.gguf"),
            AgentStateApplicationStatus::Applied,
        );

        assert_eq!(agent_status_label(&snapshot), "OK");
        Ok(())
    }

    #[test]
    fn label_says_pending_when_state_is_fresh() -> Result<()> {
        let snapshot = snapshot_with(
            0,
            0,
            Some("/models/model.gguf"),
            AgentStateApplicationStatus::Fresh,
        );

        assert_eq!(agent_status_label(&snapshot), "Pending");
        Ok(())
    }

    #[test]
    fn label_says_retrying_when_state_is_attempted_and_retrying() -> Result<()> {
        let snapshot = snapshot_with(
            0,
            0,
            Some("/models/model.gguf"),
            AgentStateApplicationStatus::AttemptedAndRetrying,
        );

        assert_eq!(agent_status_label(&snapshot), "Retrying");
        Ok(())
    }

    #[test]
    fn label_warns_about_stuck_when_state_is_stuck() -> Result<()> {
        let snapshot = snapshot_with(
            0,
            0,
            Some("/models/model.gguf"),
            AgentStateApplicationStatus::Stuck,
        );

        assert_eq!(agent_status_label(&snapshot), "Retrying, but seems stuck?");
        Ok(())
    }

    #[test]
    fn label_asks_for_help_when_state_is_attempted_and_not_appliable() -> Result<()> {
        let snapshot = snapshot_with(
            0,
            0,
            Some("/models/model.gguf"),
            AgentStateApplicationStatus::AttemptedAndNotAppliable,
        );

        assert_eq!(agent_status_label(&snapshot), "Needs your help");
        Ok(())
    }
}
