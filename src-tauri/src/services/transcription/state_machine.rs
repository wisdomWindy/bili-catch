use crate::models::TranscribeTaskStatus;

pub fn can_transition(from: TranscribeTaskStatus, to: TranscribeTaskStatus) -> bool {
    use TranscribeTaskStatus::*;
    matches!(
        (from, to),
        (Pending, Preprocessing | Cancelled | Failed)
            | (Preprocessing, LoadingModel | Cancelled | Failed)
            | (LoadingModel, Transcribing | Cancelled | Failed)
            | (Transcribing, Processing | Cancelled | Failed)
            | (Processing, Saving | Cancelled | Failed)
            | (Saving, Completed | Failed)
    )
}

pub fn can_retry(status: TranscribeTaskStatus) -> bool {
    matches!(
        status,
        TranscribeTaskStatus::Failed | TranscribeTaskStatus::Cancelled
    )
}

pub fn is_terminal(status: TranscribeTaskStatus) -> bool {
    matches!(
        status,
        TranscribeTaskStatus::Completed
            | TranscribeTaskStatus::Failed
            | TranscribeTaskStatus::Cancelled
    )
}

pub fn is_running(status: TranscribeTaskStatus) -> bool {
    matches!(
        status,
        TranscribeTaskStatus::Preprocessing
            | TranscribeTaskStatus::LoadingModel
            | TranscribeTaskStatus::Transcribing
            | TranscribeTaskStatus::Processing
            | TranscribeTaskStatus::Saving
    )
}
