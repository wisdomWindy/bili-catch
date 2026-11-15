use std::sync::Arc;

use async_trait::async_trait;

use crate::infrastructure::bilibili::{
    adapt_audio_metadata, adapt_audio_source_candidates, adapt_bangumi_result,
    adapt_collection_result, adapt_parse_result, adapt_video_source_candidates,
    bangumi_episode_availability, normalize_input, select_part, BilibiliClient, BilibiliPort,
    Clock, NormalizedInput, PlayData, PortError, SeasonData, SystemClock, VideoId, WbiKey,
    WbiKeyCache,
};
use crate::models::{
    AppError, AppErrorCode, AudioOutputProfile, Availability, DownloadMode, ParseVideoResult,
    UnavailableReason, VideoSource,
};
use crate::services::audio::{
    select_audio_source, AudioMetadata, AudioSourceBundle, AudioSourcePort, AudioSourceRequest,
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

        match &normalized.video_id {
            VideoId::Episode(ep_id) => {
                let season = self
                    .port
                    .fetch_bangumi_season(Some(*ep_id), None, &auth)
                    .await?;
                let play = self
                    .bangumi_capability_play(&season, Some(*ep_id), &auth)
                    .await;
                return adapt_bangumi_result(season, Some(*ep_id), play);
            }
            VideoId::Season(season_id) => {
                let season = self
                    .port
                    .fetch_bangumi_season(None, Some(*season_id), &auth)
                    .await?;
                let play = self.bangumi_capability_play(&season, None, &auth).await;
                return adapt_bangumi_result(season, None, play);
            }
            _ => {}
        }

        let view = self.port.fetch_view(&normalized.video_id, &auth).await?;
        let part = select_part(&view, normalized.requested_page)?;
        let play = self
            .fetch_playurl_with_retry(&normalized.video_id, part.cid, &auth)
            .await?;

        if view.ugc_season.is_some() {
            return adapt_collection_result(view, play, normalized.requested_page);
        }

        adapt_parse_result(view, play, normalized.requested_page)
    }

    /// Capabilities for a season come from one representative episode; failures degrade to none.
    async fn bangumi_capability_play(
        &self,
        season: &SeasonData,
        requested_ep: Option<u64>,
        auth: &ValidatedAuthContext,
    ) -> Option<PlayData> {
        let preferred = requested_ep
            .and_then(|ep_id| {
                season
                    .episodes
                    .iter()
                    .find(|episode| episode.ep_id == ep_id)
            })
            .filter(|episode| bangumi_episode_availability(episode).0 == Availability::Playable);
        let episode = preferred.or_else(|| {
            season.episodes.iter().find(|episode| {
                episode.section_type == 0
                    && bangumi_episode_availability(episode).0 == Availability::Playable
            })
        })?;
        self.port
            .fetch_bangumi_playurl(episode.ep_id, episode.cid, auth)
            .await
            .ok()
    }

    async fn fetch_bangumi_source(
        &self,
        ep_id: Option<u64>,
        cid: u64,
        auth: &ValidatedAuthContext,
    ) -> Result<(PlayData, SeasonData), AppError> {
        let ep_id = ep_id.ok_or_else(|| {
            AppError::new(
                AppErrorCode::E003,
                "The task is missing its bangumi episode id",
            )
        })?;
        let season = self
            .port
            .fetch_bangumi_season(Some(ep_id), None, auth)
            .await?;
        let episode = season
            .episodes
            .iter()
            .find(|episode| episode.ep_id == ep_id && episode.cid == cid)
            .ok_or_else(|| {
                AppError::new(AppErrorCode::E004, "The requested episode is unavailable")
            })?;
        let (availability, reason) = bangumi_episode_availability(episode);
        if availability == Availability::Unavailable {
            return Err(permission_error(reason));
        }
        let play = self.port.fetch_bangumi_playurl(ep_id, cid, auth).await?;
        Ok((play, season))
    }
}

fn permission_error(reason: Option<UnavailableReason>) -> AppError {
    match reason {
        Some(UnavailableReason::LoginRequired) => AppError::new(
            AppErrorCode::E005,
            "Authentication is required for this episode",
        ),
        Some(UnavailableReason::VipRequired) => AppError::new(
            AppErrorCode::E006,
            "This episode requires a premium account",
        ),
        Some(UnavailableReason::Paid) => {
            AppError::new(AppErrorCode::E006, "This episode requires a purchase")
        }
        Some(UnavailableReason::RegionLocked) => AppError::new(
            AppErrorCode::E006,
            "This episode is not available in your region",
        ),
        Some(UnavailableReason::DownloadDisabled) => {
            AppError::new(AppErrorCode::E006, "This episode cannot be downloaded")
        }
        _ => AppError::new(
            AppErrorCode::E006,
            "This episode is unavailable for the current account",
        ),
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
        let auth = self
            .auth
            .validated_context()
            .await
            .map_err(map_audio_source_error)?;
        let (play, metadata) = if request.source == VideoSource::Bangumi {
            let (play, season) = self
                .fetch_bangumi_source(request.ep_id, request.cid, &auth)
                .await
                .map_err(map_audio_source_error)?;
            (
                play,
                AudioMetadata {
                    title: season.title,
                    uploader: String::new(),
                    cover_url: season.cover,
                },
            )
        } else {
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
            (play, adapt_audio_metadata(&view)?)
        };
        let source = select_audio_source(
            &adapt_audio_source_candidates(&play),
            request.output_profile,
            auth.is_authenticated(),
        )?;
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
        let auth = self
            .auth
            .validated_context()
            .await
            .map_err(map_video_source_error)?;
        let play = if request.source == VideoSource::Bangumi {
            self.fetch_bangumi_source(request.ep_id, request.cid, &auth)
                .await
                .map_err(map_video_source_error)?
                .0
        } else {
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
            self.fetch_playurl_with_retry(&video_id, request.cid, &auth)
                .await
                .map_err(map_video_source_error)?
        };
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
    use crate::infrastructure::bilibili::{PlayData, SeasonData, ViewData};
    use crate::models::{AudioOutputProfile, Availability, ContainerKind, VideoSource};
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
          "bvid":"BV1xx411c7BF","aid":170001,"title":"Fixture","pic":"http://i0.hdslb.com/a.jpg",
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

        async fn fetch_bangumi_season(
            &self,
            _ep_id: Option<u64>,
            _season_id: Option<u64>,
            _auth: &ValidatedAuthContext,
        ) -> Result<SeasonData, AppError> {
            Err(AppError::new(AppErrorCode::E004, "fixture has no season"))
        }

        async fn fetch_bangumi_playurl(
            &self,
            _ep_id: u64,
            _cid: u64,
            _auth: &ValidatedAuthContext,
        ) -> Result<PlayData, AppError> {
            Ok(play_fixture())
        }
    }

    struct StaticPort {
        view: ViewData,
        season: Option<SeasonData>,
    }

    #[async_trait]
    impl BilibiliPort for StaticPort {
        async fn resolve_short_url(&self, _url: &str) -> Result<String, AppError> {
            Err(AppError::internal("unused in this fixture"))
        }

        async fn fetch_view(
            &self,
            _video_id: &VideoId,
            _auth: &ValidatedAuthContext,
        ) -> Result<ViewData, AppError> {
            Ok(self.view.clone())
        }

        async fn fetch_bangumi_season(
            &self,
            _ep_id: Option<u64>,
            _season_id: Option<u64>,
            _auth: &ValidatedAuthContext,
        ) -> Result<SeasonData, AppError> {
            self.season
                .clone()
                .ok_or_else(|| AppError::new(AppErrorCode::E004, "fixture has no season"))
        }

        async fn fetch_bangumi_playurl(
            &self,
            _ep_id: u64,
            _cid: u64,
            _auth: &ValidatedAuthContext,
        ) -> Result<PlayData, AppError> {
            Ok(play_fixture())
        }

        async fn fetch_wbi_key(&self, _auth: &ValidatedAuthContext) -> Result<WbiKey, AppError> {
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
            Ok(play_fixture())
        }
    }

    #[test]
    fn reserves_bangumi_targets_before_any_metadata_request() {
        tauri::async_runtime::block_on(async {
            let port = Arc::new(MockPort::new(0));
            let auth = Arc::new(FakeAuthContext::new(0));
            let service = ParserService::new(port.clone(), Arc::new(FixedClock), auth);

            let error = service
                .parse("https://www.bilibili.com/bangumi/play/ep733316")
                .await
                .expect_err("bangumi targets must not be parsed as UGC");

            assert_eq!(
                serde_json::to_value(error).unwrap()["code"],
                "E004",
                "bangumi target must not be reported as invalid input"
            );
            assert_eq!(port.view_calls.load(Ordering::SeqCst), 0);
        });
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
    fn parses_bangumi_episode_through_the_season_port() {
        tauri::async_runtime::block_on(async {
            let season: SeasonData = serde_json::from_str(
                r#"{
                  "season_id": 9015689, "title": "AI信息分享",
                  "cover": "https://i0.hdslb.com/season.jpg",
                  "episodes": [
                    {"ep_id":733316,"aid":1,"bvid":"BV1kw8b6bEeL","cid":41191148952,
                     "title":"1","long_title":"第1话","duration":1203160,"badge":"","section_type":0,
                     "rights":{"allow_download":1}},
                    {"ep_id":733317,"aid":2,"bvid":"BV1c94Z6iE9N","cid":41335523146,
                     "title":"2","long_title":"第2话","duration":907000,"badge":"会员","section_type":0,
                     "rights":{"allow_download":1}},
                    {"ep_id":733318,"aid":3,"bvid":"BV1preview","cid":41335529999,
                     "title":"预告","long_title":"预告","duration":30000,"badge":"预告","section_type":1,
                     "rights":{"allow_download":1}}
                  ]
                }"#,
            )
            .unwrap();
            let view: ViewData = serde_json::from_str(
                r#"{"bvid":"","aid":0,"title":"","pic":"","duration":0,"owner":{"name":""},"pages":[]}"#,
            )
            .unwrap();
            let service = ParserService::new(
                Arc::new(StaticPort {
                    view,
                    season: Some(season),
                }),
                Arc::new(FixedClock),
                Arc::new(FakeAuthContext::new(0)),
            );

            let result = service
                .parse("https://www.bilibili.com/bangumi/play/ep733316")
                .await
                .unwrap();

            assert_eq!(result.source, VideoSource::Bangumi);
            assert_eq!(result.parts.len(), 2, "preview episodes must be filtered");
            assert_eq!(result.requested_cid, Some(41191148952));
            assert_eq!(result.parts[0].duration_seconds, 1203);
            assert_eq!(result.parts[1].availability, Availability::Unavailable);
            assert!(
                !result.qualities.is_empty(),
                "capabilities must come from the representative episode"
            );
            assert_eq!(
                result.container.as_ref().map(|item| item.id.as_str()),
                Some("9015689")
            );
            assert_eq!(result.bvid.as_deref(), Some("BV1kw8b6bEeL"));
        });
    }

    #[test]
    fn expands_a_collection_into_units_from_ugc_season() {
        tauri::async_runtime::block_on(async {
            let view: ViewData = serde_json::from_str(
                r#"{
                  "bvid":"BV1kw8b6bEeL","aid":117144863906774,"title":"Entry",
                  "pic":"http://i2.hdslb.com/cover.jpg","duration":1247,
                  "owner":{"name":"Aisland"},
                  "pages":[{"cid":41191148952,"page":1,"part":"Entry","duration":1247}],
                  "ugc_season":{
                    "id":9015689,"title":"AI信息分享","cover":"https://archive.biliimg.com/season.jpg",
                    "sections":[{"episodes":[
                      {"bvid":"BV1kw8b6bEeL","cid":41191148952,"title":"ep1","arc":{"duration":1247},
                       "pages":[
                         {"cid":41191148952,"page":1,"part":"P1","duration":600},
                         {"cid":41191148953,"page":2,"part":"P2","duration":647}
                       ]},
                      {"bvid":"BV1c94Z6iE9N","cid":41335523146,"title":"ep2","arc":{"duration":907},
                       "pages":[]}
                    ]}]
                  }
                }"#,
            )
            .unwrap();
            let service = ParserService::new(
                Arc::new(StaticPort { view, season: None }),
                Arc::new(FixedClock),
                Arc::new(FakeAuthContext::new(0)),
            );

            let result = service.parse("BV1kw8b6bEeL").await.unwrap();

            assert_eq!(result.source, VideoSource::Ugc);
            assert_eq!(result.parts.len(), 3);
            assert_eq!(
                result.container.as_ref().map(|item| item.kind),
                Some(ContainerKind::Season)
            );
            assert_eq!(result.parts[0].bvid, "BV1kw8b6bEeL");
            assert_eq!(result.parts[0].part_count, 2);
            assert_eq!(result.parts[2].bvid, "BV1c94Z6iE9N");
            assert_eq!(result.parts[2].group_index, 2);
            assert_eq!(result.title, "AI信息分享");
        });
    }

    fn empty_view() -> ViewData {
        serde_json::from_str(
            r#"{"bvid":"","aid":0,"title":"","pic":"","duration":0,"owner":{"name":""},"pages":[]}"#,
        )
        .unwrap()
    }

    fn season_fixture() -> SeasonData {
        serde_json::from_str(
            r#"{
              "season_id": 9015689, "title": "AI信息分享", "cover": "https://i0.hdslb.com/season.jpg",
              "episodes": [
                {"ep_id":733316,"aid":1,"bvid":"BV1kw8b6bEeL","cid":41191148952,
                 "title":"1","long_title":"第1话","duration":1203160,"badge":"","section_type":0,
                 "rights":{"allow_download":1}},
                {"ep_id":733317,"aid":2,"bvid":"BV1c94Z6iE9N","cid":41335523146,
                 "title":"2","long_title":"第2话","duration":907000,"badge":"会员","section_type":0,
                 "rights":{"allow_download":1}}
              ]
            }"#,
        )
        .unwrap()
    }

    #[test]
    fn resolves_bangumi_sources_through_the_pgc_port() {
        tauri::async_runtime::block_on(async {
            let service = ParserService::new(
                Arc::new(StaticPort {
                    view: empty_view(),
                    season: Some(season_fixture()),
                }),
                Arc::new(FixedClock),
                Arc::new(FakeAuthContext::new(0)),
            );
            let request = VideoSourceRequest {
                bvid: "BV1kw8b6bEeL".into(),
                cid: 41191148952,
                source: VideoSource::Bangumi,
                ep_id: Some(733316),
                mode: DownloadMode::VideoOnly,
                quality_id: "32".into(),
                codec: crate::models::VideoCodec::Avc,
            };

            let bundle = VideoSourcePort::resolve(&service, &request).await.unwrap();

            assert_eq!(bundle.video.id, 32);
        });
    }

    #[test]
    fn rejects_unavailable_bangumi_episodes_before_media_fetch() {
        tauri::async_runtime::block_on(async {
            let service = ParserService::new(
                Arc::new(StaticPort {
                    view: empty_view(),
                    season: Some(season_fixture()),
                }),
                Arc::new(FixedClock),
                Arc::new(FakeAuthContext::new(0)),
            );
            let request = AudioSourceRequest {
                bvid: "BV1c94Z6iE9N".into(),
                cid: 41335523146,
                source: VideoSource::Bangumi,
                ep_id: Some(733317),
                output_profile: AudioOutputProfile::Mp3 { bitrate_kbps: 320 },
            };

            let error = match AudioSourcePort::resolve(&service, &request).await {
                Ok(_) => panic!("premium episodes must be rejected before streaming"),
                Err(error) => error,
            };

            assert_eq!(serde_json::to_value(error).unwrap()["code"], "E006");
        });
    }

    #[test]
    fn rejects_bangumi_requests_without_episode_id() {
        tauri::async_runtime::block_on(async {
            let service = ParserService::new(
                Arc::new(StaticPort {
                    view: empty_view(),
                    season: Some(season_fixture()),
                }),
                Arc::new(FixedClock),
                Arc::new(FakeAuthContext::new(0)),
            );
            let request = AudioSourceRequest {
                bvid: "BV1kw8b6bEeL".into(),
                cid: 41191148952,
                source: VideoSource::Bangumi,
                ep_id: None,
                output_profile: AudioOutputProfile::Mp3 { bitrate_kbps: 320 },
            };

            let error = match AudioSourcePort::resolve(&service, &request).await {
                Ok(_) => panic!("missing episode ids must be rejected"),
                Err(error) => error,
            };

            assert_eq!(serde_json::to_value(error).unwrap()["code"], "E003");
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
                source: VideoSource::Ugc,
                ep_id: None,
                output_profile: AudioOutputProfile::Mp3 { bitrate_kbps: 320 },
            };

            let first = AudioSourcePort::resolve(&service, &request).await.unwrap();
            let second = AudioSourcePort::resolve(&service, &request).await.unwrap();

            assert_eq!(first.source.media.id, 30216);
            assert_eq!(first.metadata.cover_url, "https://i0.hdslb.com/a.jpg");
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
                source: VideoSource::Ugc,
                ep_id: None,
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
