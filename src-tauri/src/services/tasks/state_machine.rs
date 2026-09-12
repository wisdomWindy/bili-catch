use crate::models::{TaskAction, TaskStatus};

pub fn actions_for_status(status: TaskStatus, can_cancel_processing: bool) -> Vec<TaskAction> {
    match status {
        TaskStatus::Queued => vec![TaskAction::Cancel],
        TaskStatus::Downloading => vec![TaskAction::Pause, TaskAction::Cancel],
        TaskStatus::Paused => vec![TaskAction::Resume, TaskAction::Cancel],
        TaskStatus::Processing if can_cancel_processing => vec![TaskAction::Cancel],
        TaskStatus::Processing => vec![],
        TaskStatus::Failed => vec![TaskAction::Retry, TaskAction::Delete],
        TaskStatus::Completed | TaskStatus::Cancelled => vec![TaskAction::Delete],
    }
}
