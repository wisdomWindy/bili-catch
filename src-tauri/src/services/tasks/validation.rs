use crate::models::{AppError, AudioOutputProfile, DownloadMode, DownloadTaskDraft, VideoSource};

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
            || !valid_source_fields(&draft)
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

fn valid_source_fields(draft: &DownloadTaskDraft) -> bool {
    match draft.source {
        VideoSource::Bangumi => draft.ep_id.is_some_and(|ep_id| ep_id > 0),
        VideoSource::Ugc => draft.ep_id.is_none(),
    }
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

#[cfg(test)]
mod tests {
    use super::validate_create_request;
    use crate::models::{
        AppError, AudioFormat, ContainerKind, DownloadMode, DownloadTaskDraft, VideoCodec,
        VideoSource,
    };

    fn draft(source: VideoSource, ep_id: Option<u64>) -> DownloadTaskDraft {
        DownloadTaskDraft {
            canonical_url: "https://www.bilibili.com/video/BV1xx411c7BF".into(),
            bvid: "BV1xx411c7BF".into(),
            cid: 1001,
            page: 1,
            part_title: "P1".into(),
            video_title: "Example".into(),
            part_count: 1,
            source,
            container_kind: ContainerKind::Video,
            ep_id,
            group_index: 1,
            mode: DownloadMode::VideoOnly,
            output_dir: "Downloads".into(),
            quality_id: Some("80".into()),
            codec: Some(VideoCodec::Avc),
            audio_format: None,
            audio_bitrate_id: None,
            auto_transcribe: false,
            transcription_preset: None,
        }
    }

    #[test]
    fn accepts_consistent_source_fields() {
        assert!(validate_create_request("batch-1", vec![draft(VideoSource::Ugc, None)], 0).is_ok());
        assert!(validate_create_request(
            "batch-2",
            vec![draft(VideoSource::Bangumi, Some(733316))],
            0
        )
        .is_ok());
    }

    #[test]
    fn rejects_inconsistent_source_fields() {
        for (source, ep_id) in [
            (VideoSource::Bangumi, None),
            (VideoSource::Bangumi, Some(0)),
            (VideoSource::Ugc, Some(733316)),
        ] {
            let error = validate_create_request("batch-1", vec![draft(source, ep_id)], 0)
                .expect_err("inconsistent source fields must be rejected");
            assert_eq!(error.code, AppError::internal("fixture").code);
        }
    }

    #[test]
    fn audio_drafts_still_require_a_matching_audio_profile() {
        let mut value = draft(VideoSource::Ugc, None);
        value.mode = DownloadMode::AudioOnly;
        value.quality_id = None;
        value.codec = None;
        value.audio_format = Some(AudioFormat::Mp3);
        value.audio_bitrate_id = Some("320".into());
        assert!(validate_create_request("batch-1", vec![value], 0).is_ok());
    }
}
