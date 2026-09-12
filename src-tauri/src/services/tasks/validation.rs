use crate::models::{AppError, AudioOutputProfile, DownloadMode, DownloadTaskDraft};

pub const TASK_CAPACITY: usize = 100;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedCreateRequest {
    pub request_id: String,
    pub drafts: Vec<DownloadTaskDraft>,
}

pub fn validate_create_request(
    request_id: &str,
    drafts: Vec<DownloadTaskDraft>,
    current_count: usize,
) -> Result<ValidatedCreateRequest, AppError> {
    let request_id = request_id.trim();
    if request_id.is_empty()
        || request_id.len() > 80
        || !request_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
    {
        return Err(AppError::internal("Invalid task request id"));
    }
    if drafts.is_empty() {
        return Err(AppError::internal("Task batch cannot be empty"));
    }
    if current_count.saturating_add(drafts.len()) > TASK_CAPACITY {
        let mut error = AppError::internal("Task queue is full");
        error.details = Some("TASK_QUEUE_FULL".into());
        return Err(error);
    }

    let mut normalized = Vec::with_capacity(drafts.len());
    for mut draft in drafts {
        draft.output_dir = draft.output_dir.trim().to_owned();
        if draft.output_dir.is_empty()
            || draft.video_title.trim().is_empty()
            || draft.part_count == 0
            || draft.page == 0
            || draft.page > draft.part_count
            || !valid_media_fields(&draft)
        {
            return Err(AppError::internal("Invalid download task draft"));
        }
        normalized.push(draft);
    }

    Ok(ValidatedCreateRequest {
        request_id: request_id.to_owned(),
        drafts: normalized,
    })
}

fn valid_media_fields(draft: &DownloadTaskDraft) -> bool {
    let has_video = draft.quality_id.as_deref().is_some_and(|quality_id| {
        !quality_id.is_empty() && quality_id.bytes().all(|byte| byte.is_ascii_digit())
    }) && draft.codec.is_some();
    let has_audio = draft.audio_format.is_some() && draft.audio_bitrate_id.is_some();
    match draft.mode {
        DownloadMode::VideoAudio | DownloadMode::VideoOnly => has_video && !has_audio,
        DownloadMode::AudioOnly => {
            !has_video
                && has_audio
                && AudioOutputProfile::try_from_selection(
                    draft.audio_format.clone().expect("checked above"),
                    draft.audio_bitrate_id.as_deref().expect("checked above"),
                )
                .is_ok()
        }
    }
}
