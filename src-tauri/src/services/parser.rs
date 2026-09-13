use std::sync::Arc;

use async_trait::async_trait;

use crate::infrastructure::bilibili::{
    adapt_audio_metadata, adapt_audio_source_candidates, adapt_parse_result,
    adapt_video_source_candidates, normalize_input, select_part, validate_media_url,
    BilibiliClient, BilibiliPort, Clock, NormalizedInput, PortError, SystemClock, VideoId, WbiKey,
    WbiKeyCache,
};
use crate::models::{AppError, AppErrorCode, AudioOutputProfile, DownloadMode, ParseVideoResult};
use crate::services::audio::{
    select_audio_source, AudioSourceBundle, AudioSourcePort, AudioSourceRequest,
};
use crate::services::auth::context::{AuthContextProvider, ValidatedAuthContext};
use crate::services::video::{
    select_video_source, VideoSourceBundle, VideoSourcePort, VideoSourceRequest,
};

pub(crate) struct ParserService {
    port: Arc<dyn BilibiliPort>,
    clock: Arc<dyn Clock>,
    auth: Arc<dyn AuthContextProvider>,
    wbi_cache: WbiKeyCache,
}

impl ParserService {
    pub(crate) fn production(auth: Arc<dyn AuthContextProvider>) -> Result<Self, AppError> {
        Ok(Self::new(
            Arc::new(BilibiliClient::new()?),
            Arc::new(SystemClock),
            auth,
        ))
    }

    fn new(
        port: Arc<dyn BilibiliPort>,
        clock: Arc<dyn Clock>,
        auth: Arc<dyn AuthContextProvider>,
    ) -> Self {
        Self {
            port,
            clock,
            auth,
            wbi_cache: WbiKeyCache::new(),
        }
    }

    async fn resolved_input(&self, input: &str) -> Result<NormalizedInput, AppError> {
        let normalized = normalize_input(input)?;
        match &normalized.video_id {
            VideoId::ShortUrl(url) => {
                let resolved = self.port.resolve_short_url(url).await?;
                let resolved = normalize_input(&resolved)?;
                if matches!(resolved.video_id, VideoId::ShortUrl(_)) {
                    return Err(AppError::new(
                        AppErrorCode::E003,
                        "The short link did not resolve to a video",
                    ));
                }
                Ok(resolved)
            }
            _ => Ok(normalized),
        }
    }

    async fn wbi_key(&self, now: u64, auth: &ValidatedAuthContext) -> Result<WbiKey, AppError> {
        if let Some(key) = self.wbi_cache.get(now) {
            return Ok(key);
        }
        let key = self.port.fetch_wbi_key(auth).await?;
        self.wbi_cache.put(key.clone(), now);
        Ok(key)
    }

    async fn fetch_playurl_with_retry(
        &self,
        video_id: &VideoId,
        cid: u64,
        auth: &ValidatedAuthContext,
    ) -> Result<crate::infrastructure::bilibili::PlayData, AppError> {
        let now = self.clock.now_seconds();
        let key = self.wbi_key(now, auth).await?;
        match self
            .port
            .fetch_playurl(video_id, cid, &key, now, auth)
            .await
        {
            Ok(play) => Ok(play),
            Err(PortError::Failure(error)) => Err(error),
            Err(PortError::Signature) => {
                self.wbi_cache.invalidate();
                let retry_key = self.wbi_key(now, auth).await?;
                match self
                    .port
                    .fetch_playurl(video_id, cid, &retry_key, now, auth)
                    .await
                {
                    Ok(play) => Ok(play),
                    Err(PortError::Failure(error)) => Err(error),
                    Err(PortError::Signature) => Err(AppError::new(
                        AppErrorCode::E002,
                        "Bilibili rejected the refreshed request signature",
                    )),
                }
            }
        }
    }

    pub async fn parse(&self, input: &str) -> Result<ParseVideoResult, AppError> {
        let normalized = self.resolved_input(input).await?;
        let auth = self.auth.validated_context().await?;
        let view = self.port.fetch_view(&normalized.video_id, &auth).await?;
        let part = select_part(&view, normalized.requested_page)?;
        let play = self
            .fetch_playurl_with_retry(&normalized.video_id, part.cid, &auth)
            .await?;

        adapt_parse_result(view, play, normalized.requested_page)
    }
}

fn map_audio_source_error(error: AppError) -> AppError {
    match error.code {
        AppErrorCode::E001 | AppErrorCode::E002 => AppError::new(
            AppErrorCode::E009,
            "The audio source request was interrupted",
        ),
        _ => error,
    }
}

fn map_video_source_error(error: AppError) -> AppError {
    match error.code {
        AppErrorCode::E001 | AppErrorCode::E002 => AppError::new(
            AppErrorCode::E009,
            "The video source request was interrupted",
        ),
        _ => error,
    }
}

#[async_trait]
impl AudioSourcePort for ParserService {
    async fn resolve(&self, request: &AudioSourceRequest) -> Result<AudioSourceBundle, AppError> {
        let normalized = normalize_input(&request.bvid)?;
        let video_id = match normalized.video_id {
            VideoId::Bvid(value) if value == request.bvid => VideoId::Bvid(value),
            _ => {
                return Err(AppError::new(
                    AppErrorCode::E003,
                    "The audio task contains an invalid video identifier",
                ))
            }
        };
        let auth = self
            .auth
            .validated_context()
            .await
            .map_err(map_audio_source_error)?;
        let view = self
            .port
            .fetch_view(&video_id, &auth)
            .await
            .map_err(map_audio_source_error)?;
        if !view.pages.iter().any(|part| part.cid == request.cid) {
            return Err(AppError::new(
                AppErrorCode::E004,
                "The requested audio part is unavailable",
            ));
        }
        let play = self
            .fetch_playurl_with_retry(&video_id, request.cid, &auth)
            .await
            .map_err(map_audio_source_error)?;
        let source = select_audio_source(
            &adapt_audio_source_candidates(&play),
            request.output_profile,
            auth.is_authenticated(),
        )?;
        let metadata = adapt_audio_metadata(&view);
        validate_media_url(&metadata.cover_url)?;
        Ok(AudioSourceBundle { source, metadata })
    }
}

#[async_trait]
impl VideoSourcePort for ParserService {
    async fn resolve(&self, request: &VideoSourceRequest) -> Result<VideoSourceBundle, AppError> {
        if request.mode == DownloadMode::AudioOnly {
            return Err(AppError::new(
                AppErrorCode::E003,
                "The video source request has an invalid mode",
            ));
        }
        let normalized = normalize_input(&request.bvid).map_err(map_video_source_error)?;
        let video_id = match normalized.video_id {
            VideoId::Bvid(value) if value == request.bvid => VideoId::Bvid(value),
            _ => {
                return Err(AppError::new(
                    AppErrorCode::E003,
                    "The video task contains an invalid video identifier",
                ))
            }
        };
        let auth = self
            .auth
            .validated_context()
            .await
            .map_err(map_video_source_error)?;
        let view = self
            .port
            .fetch_view(&video_id, &auth)
            .await
            .map_err(map_video_source_error)?;
        if !view.pages.iter().any(|part| part.cid == request.cid) {
            return Err(AppError::new(
                AppErrorCode::E004,
                "The requested video part is unavailable",
            ));
        }
        let play = self
            .fetch_playurl_with_retry(&video_id, request.cid, &auth)
            .await
            .map_err(map_video_source_error)?;
        let video = select_video_source(
            &adapt_video_source_candidates(&play),
            &request.quality_id,
            request.codec.clone(),
            auth.is_authenticated(),
        )
        .map_err(map_video_source_error)?;
        let audio = if request.mode == DownloadMode::VideoAudio {
            Some(
                select_audio_source(
                    &adapt_audio_source_candidates(&play),
                    AudioOutputProfile::M4aOriginal,
                    auth.is_authenticated(),
                )?
                .media,
            )
        } else {
            None
        };
        Ok(VideoSourceBundle { video, audio })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

    use async_trait::async_trait;

    use super::*;
    use crate::infrastructure::bilibili::{PlayData, ViewData};
    use crate::models::AudioOutputProfile;
    use crate::services::audio::{AudioSourcePort, AudioSourceRequest};
    use crate::services::auth::context::{AuthContextProvider, ValidatedAuthContext};
    use crate::services::video::{VideoSourcePort, VideoSourceRequest};

    struct FixedClock;

    impl Clock for FixedClock {
        fn now_seconds(&self) -> u64 {
            1_700_000_000
        }
    }

    struct MockPort {
        view_calls: AtomicUsize,
        key_calls: AtomicUsize,
        play_calls: AtomicUsize,
        rejected_signature_attempts: usize,
    }

    struct FakeAuthContext {
        revision: AtomicU64,
        calls: AtomicUsize,
    }

    struct FailingAuthContext;

    impl FakeAuthContext {
        fn new(revision: u64) -> Self {
            Self {
                revision: AtomicU64::new(revision),
                calls: AtomicUsize::new(0),
            }
        }
    }

    #[async_trait]
    impl AuthContextProvider for FakeAuthContext {
        async fn validated_context(&self) -> Result<ValidatedAuthContext, AppError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(ValidatedAuthContext::anonymous())
        }
    }

    #[async_trait]
    impl AuthContextProvider for FailingAuthContext {
        async fn validated_context(&self) -> Result<ValidatedAuthContext, AppError> {
            Err(AppError::new(
                AppErrorCode::E001,
                "fixture validation failure",
            ))
        }
    }

    impl MockPort {
        fn new(rejected_signature_attempts: usize) -> Self {
            Self {
                view_calls: AtomicUsize::new(0),
                key_calls: AtomicUsize::new(0),
                play_calls: AtomicUsize::new(0),
                rejected_signature_attempts,
            }
        }
    }

    fn view_fixture() -> ViewData {
        serde_json::from_str(r#"{
          "bvid":"BV1xx411c7BF","aid":170001,"title":"Fixture","pic":"https://i0.hdslb.com/a.jpg",
          "duration":90,"owner":{"name":"Owner"},"pages":[{"cid":1001,"page":1,"part":"P1","duration":90}]
        }"#).unwrap()
    }

    fn play_fixture() -> PlayData {
        serde_json::from_str(
            r#"{
          "accept_quality":[32],"accept_description":["480P"],
          "dash":{"video":[{"id":32,"baseUrl":"https://a.bilivideo.com/video.m4s","backupUrl":[],"bandwidth":1000000,"mimeType":"video/mp4","codecs":"avc1.64001f"}],"audio":[{
            "id":30216,"baseUrl":"https://a.bilivideo.com/audio.m4s","backupUrl":[],
            "bandwidth":64000,"mimeType":"audio/mp4","codecs":"mp4a.40.2"
          }]}
        }"#,
        )
        .unwrap()
    }

    #[async_trait]
    impl BilibiliPort for MockPort {
        async fn resolve_short_url(&self, _url: &str) -> Result<String, AppError> {
            Ok("https://www.bilibili.com/video/BV1xx411c7BF".into())
        }

        async fn fetch_view(
            &self,
            _video_id: &VideoId,
            _auth: &ValidatedAuthContext,
        ) -> Result<ViewData, AppError> {
            self.view_calls.fetch_add(1, Ordering::SeqCst);
            Ok(view_fixture())
        }

        async fn fetch_wbi_key(&self, _auth: &ValidatedAuthContext) -> Result<WbiKey, AppError> {
            self.key_calls.fetch_add(1, Ordering::SeqCst);
            Ok(WbiKey {
                mixin_key: "abcdefghijklmnopqrstuvwxyz123456".into(),
            })
        }

        async fn fetch_playurl(
            &self,
            _video_id: &VideoId,
            _cid: u64,
            _key: &WbiKey,
            _timestamp: u64,
            _auth: &ValidatedAuthContext,
        ) -> Result<PlayData, PortError> {
            let call = self.play_calls.fetch_add(1, Ordering::SeqCst);
            if call < self.rejected_signature_attempts {
                Err(PortError::Signature)
            } else {
                Ok(play_fixture())
            }
        }
    }

    #[test]
    fn retries_one_signature_failure_and_refreshes_each_parse() {
        tauri::async_runtime::block_on(async {
            let port = Arc::new(MockPort::new(1));
            let auth = Arc::new(FakeAuthContext::new(0));
            let service = ParserService::new(port.clone(), Arc::new(FixedClock), auth.clone());

            let first = service.parse("BV1xx411c7BF").await.unwrap();
            let second = service.parse("BV1xx411c7BF").await.unwrap();

            assert_eq!(first, second);
            assert_eq!(port.view_calls.load(Ordering::SeqCst), 2);
            assert_eq!(port.play_calls.load(Ordering::SeqCst), 3);
            assert_eq!(port.key_calls.load(Ordering::SeqCst), 2);
            assert_eq!(auth.calls.load(Ordering::SeqCst), 2);
        });
    }

    #[test]
    fn stops_after_two_signature_failures() {
        tauri::async_runtime::block_on(async {
            let port = Arc::new(MockPort::new(2));
            let auth = Arc::new(FakeAuthContext::new(0));
            let service = ParserService::new(port.clone(), Arc::new(FixedClock), auth);

            let error = service.parse("BV1xx411c7BF").await.unwrap_err();

            assert_eq!(serde_json::to_value(error).unwrap()["code"], "E002");
            assert_eq!(port.play_calls.load(Ordering::SeqCst), 2);
            assert_eq!(port.key_calls.load(Ordering::SeqCst), 2);
        });
    }

    #[test]
    fn refreshes_parse_after_auth_revision_changes() {
        tauri::async_runtime::block_on(async {
            let port = Arc::new(MockPort::new(0));
            let auth = Arc::new(FakeAuthContext::new(0));
            let service = ParserService::new(port.clone(), Arc::new(FixedClock), auth.clone());

            service.parse("BV1xx411c7BF").await.unwrap();
            auth.revision.store(1, Ordering::SeqCst);
            service.parse("BV1xx411c7BF").await.unwrap();

            assert_eq!(auth.calls.load(Ordering::SeqCst), 2);
            assert_eq!(port.view_calls.load(Ordering::SeqCst), 2);
            assert_eq!(port.play_calls.load(Ordering::SeqCst), 2);
        });
    }

    #[test]
    fn validation_failure_stops_before_any_media_request() {
        tauri::async_runtime::block_on(async {
            let port = Arc::new(MockPort::new(0));
            let service = ParserService::new(
                port.clone(),
                Arc::new(FixedClock),
                Arc::new(FailingAuthContext),
            );

            let error = service.parse("BV1xx411c7BF").await.unwrap_err();

            assert_eq!(serde_json::to_value(error).unwrap()["code"], "E001");
            assert_eq!(port.view_calls.load(Ordering::SeqCst), 0);
            assert_eq!(port.key_calls.load(Ordering::SeqCst), 0);
            assert_eq!(port.play_calls.load(Ordering::SeqCst), 0);
        });
    }

    #[test]
    fn audio_source_resolution_refreshes_auth_view_and_playurl_for_every_attempt() {
        tauri::async_runtime::block_on(async {
            let port = Arc::new(MockPort::new(0));
            let auth = Arc::new(FakeAuthContext::new(0));
            let service = ParserService::new(port.clone(), Arc::new(FixedClock), auth.clone());
            let request = AudioSourceRequest {
                bvid: "BV1xx411c7BF".into(),
                cid: 1001,
                output_profile: AudioOutputProfile::Mp3 { bitrate_kbps: 320 },
            };

            let first = AudioSourcePort::resolve(&service, &request).await.unwrap();
            let second = AudioSourcePort::resolve(&service, &request).await.unwrap();

            assert_eq!(first.source.media.id, 30216);
            assert_eq!(second.source.media.id, 30216);
            assert_eq!(port.view_calls.load(Ordering::SeqCst), 2);
            assert_eq!(port.play_calls.load(Ordering::SeqCst), 2);
            assert_eq!(auth.calls.load(Ordering::SeqCst), 2);
        });
    }

    #[test]
    fn video_source_resolution_refreshes_auth_view_and_playurl_and_selects_m4a_audio() {
        tauri::async_runtime::block_on(async {
            let port = Arc::new(MockPort::new(0));
            let auth = Arc::new(FakeAuthContext::new(0));
            let service = ParserService::new(port.clone(), Arc::new(FixedClock), auth.clone());
            let request = VideoSourceRequest {
                bvid: "BV1xx411c7BF".into(),
                cid: 1001,
                mode: DownloadMode::VideoAudio,
                quality_id: "32".into(),
                codec: crate::models::VideoCodec::Avc,
            };

            let first = VideoSourcePort::resolve(&service, &request).await.unwrap();
            let second = VideoSourcePort::resolve(&service, &request).await.unwrap();

            assert_eq!(first.video.id, 32);
            assert_eq!(first.video.mime_type, "video/mp4");
            assert_eq!(first.audio.as_ref().unwrap().mime_type, "audio/mp4");
            assert_eq!(second.video.id, 32);
            assert_eq!(port.view_calls.load(Ordering::SeqCst), 2);
            assert_eq!(port.play_calls.load(Ordering::SeqCst), 2);
            assert_eq!(auth.calls.load(Ordering::SeqCst), 2);
        });
    }
}
