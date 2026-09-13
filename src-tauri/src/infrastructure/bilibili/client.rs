use std::time::Duration;

use async_trait::async_trait;
use reqwest::{
    header::{COOKIE, LOCATION},
    redirect::Policy,
    Client, RequestBuilder, Response,
};
use serde::de::DeserializeOwned;

use crate::models::{AppError, AppErrorCode};
use crate::services::auth::context::ValidatedAuthContext;

use super::{
    input::{normalize_input, validate_allowed_https_url},
    raw::{ApiResponse, NavData, PlayData, ViewData, WbiKey},
    wbi::{derive_mixin_key, sign_query},
    VideoId,
};

pub(super) const MAX_RESPONSE_BYTES: u64 = 4 * 1024 * 1024;

#[derive(Debug, Clone)]
pub(crate) enum PortError {
    Signature,
    Failure(AppError),
}

impl From<AppError> for PortError {
    fn from(value: AppError) -> Self {
        Self::Failure(value)
    }
}

#[async_trait]
pub(crate) trait BilibiliPort: Send + Sync {
    async fn resolve_short_url(&self, url: &str) -> Result<String, AppError>;
    async fn fetch_view(
        &self,
        video_id: &VideoId,
        auth: &ValidatedAuthContext,
    ) -> Result<ViewData, AppError>;
    async fn fetch_wbi_key(&self, auth: &ValidatedAuthContext) -> Result<WbiKey, AppError>;
    async fn fetch_playurl(
        &self,
        video_id: &VideoId,
        cid: u64,
        key: &WbiKey,
        timestamp: u64,
        auth: &ValidatedAuthContext,
    ) -> Result<PlayData, PortError>;
}

pub(crate) struct BilibiliClient {
    client: Client,
}

impl BilibiliClient {
    pub fn new() -> Result<Self, AppError> {
        let client = Client::builder()
            .redirect(Policy::none())
            .connect_timeout(Duration::from_secs(8))
            .timeout(Duration::from_secs(20))
            .user_agent("Mozilla/5.0 BiliCatch/0.1")
            .build()
            .map_err(|_| AppError::internal("Unable to initialize the HTTP client"))?;
        Ok(Self { client })
    }

    async fn decode<T: DeserializeOwned>(
        &self,
        response: Response,
    ) -> Result<ApiResponse<T>, AppError> {
        let body = read_bounded_body(response).await?;
        serde_json::from_slice(&body)
            .map_err(|_| AppError::internal("Bilibili returned an unsupported response"))
    }

    fn authenticated_get(
        &self,
        url: impl reqwest::IntoUrl,
        auth: &ValidatedAuthContext,
    ) -> RequestBuilder {
        let request = self.client.get(url);
        match auth.credential() {
            Some(credential) => request.header(COOKIE, credential.expose_cookie_header()),
            None => request,
        }
    }
}

pub(super) async fn read_bounded_body(mut response: Response) -> Result<Vec<u8>, AppError> {
    if response
        .content_length()
        .is_some_and(|length| length > MAX_RESPONSE_BYTES)
    {
        return Err(response_too_large());
    }
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(map_reqwest_error)? {
        append_bounded_chunk(&mut body, &chunk)?;
    }
    Ok(body)
}

fn response_too_large() -> AppError {
    AppError::new(AppErrorCode::E001, "The remote response is too large")
}

fn append_bounded_chunk(body: &mut Vec<u8>, chunk: &[u8]) -> Result<(), AppError> {
    if body.len().saturating_add(chunk.len()) as u64 > MAX_RESPONSE_BYTES {
        return Err(response_too_large());
    }
    body.extend_from_slice(chunk);
    Ok(())
}

pub(super) fn map_reqwest_error(error: reqwest::Error) -> AppError {
    if error.is_timeout() {
        AppError::new(AppErrorCode::E002, "The Bilibili request timed out")
    } else {
        AppError::new(AppErrorCode::E001, "Unable to reach Bilibili")
    }
}

fn map_api_error(code: i64, message: String) -> AppError {
    let app_code = match code {
        -101 => AppErrorCode::E005,
        -403 | -10403 => AppErrorCode::E006,
        -404 | 62002 | 62004 => AppErrorCode::E004,
        _ => AppErrorCode::E004,
    };
    AppError::new(
        app_code,
        if message.is_empty() {
            "Bilibili rejected the request".into()
        } else {
            message
        },
    )
}

fn append_video_id(url: &mut url::Url, video_id: &VideoId) {
    let mut query = url.query_pairs_mut();
    match video_id {
        VideoId::Bvid(bvid) => {
            query.append_pair("bvid", bvid);
        }
        VideoId::Aid(aid) => {
            query.append_pair("aid", &aid.to_string());
        }
        VideoId::ShortUrl(_) => {}
    }
}

fn redirect_destination(
    current: &url::Url,
    location: &str,
    completed_redirects: usize,
) -> Result<url::Url, AppError> {
    if completed_redirects >= 5 {
        return Err(AppError::new(
            AppErrorCode::E003,
            "The short link redirected too many times",
        ));
    }
    let next = current
        .join(location)
        .map_err(|_| AppError::new(AppErrorCode::E003, "The short link destination is invalid"))?;
    validate_allowed_https_url(next.as_str())
}

#[async_trait]
impl BilibiliPort for BilibiliClient {
    async fn resolve_short_url(&self, value: &str) -> Result<String, AppError> {
        let mut current = validate_allowed_https_url(value)?;
        for hop in 0..=5 {
            let response = self
                .client
                .get(current.clone())
                .send()
                .await
                .map_err(map_reqwest_error)?;
            if response.status().is_redirection() {
                let location = response
                    .headers()
                    .get(LOCATION)
                    .and_then(|value| value.to_str().ok())
                    .ok_or_else(|| {
                        AppError::new(
                            AppErrorCode::E003,
                            "The short link has no valid destination",
                        )
                    })?;
                current = redirect_destination(&current, location, hop)?;
                continue;
            }
            let normalized = normalize_input(current.as_str())?;
            if matches!(normalized.video_id, VideoId::ShortUrl(_)) {
                return Err(AppError::new(
                    AppErrorCode::E003,
                    "The short link did not resolve to a video",
                ));
            }
            return Ok(current.to_string());
        }
        Err(AppError::new(
            AppErrorCode::E003,
            "The short link could not be resolved",
        ))
    }

    async fn fetch_view(
        &self,
        video_id: &VideoId,
        auth: &ValidatedAuthContext,
    ) -> Result<ViewData, AppError> {
        let mut url = url::Url::parse("https://api.bilibili.com/x/web-interface/view")
            .map_err(|_| AppError::internal("The metadata endpoint is invalid"))?;
        append_video_id(&mut url, video_id);
        let response = self
            .authenticated_get(url, auth)
            .send()
            .await
            .map_err(map_reqwest_error)?;
        let envelope: ApiResponse<ViewData> = self.decode(response).await?;
        if envelope.code != 0 {
            return Err(map_api_error(envelope.code, envelope.message));
        }
        envelope
            .data
            .ok_or_else(|| AppError::internal("Bilibili metadata is missing"))
    }

    async fn fetch_wbi_key(&self, auth: &ValidatedAuthContext) -> Result<WbiKey, AppError> {
        let response = self
            .authenticated_get("https://api.bilibili.com/x/web-interface/nav", auth)
            .send()
            .await
            .map_err(map_reqwest_error)?;
        let envelope: ApiResponse<NavData> = self.decode(response).await?;
        let nav = envelope
            .data
            .ok_or_else(|| AppError::internal("Bilibili WBI metadata is missing"))?;
        derive_mixin_key(&nav.wbi_img.img_url, &nav.wbi_img.sub_url)
            .ok_or_else(|| AppError::internal("Bilibili WBI keys are invalid"))
    }

    async fn fetch_playurl(
        &self,
        video_id: &VideoId,
        cid: u64,
        key: &WbiKey,
        timestamp: u64,
        auth: &ValidatedAuthContext,
    ) -> Result<PlayData, PortError> {
        let mut parameters = vec![
            ("cid".into(), cid.to_string()),
            ("fnval".into(), "4048".into()),
            ("fourk".into(), "1".into()),
            ("qn".into(), "127".into()),
        ];
        match video_id {
            VideoId::Bvid(bvid) => parameters.push(("bvid".into(), bvid.clone())),
            VideoId::Aid(aid) => parameters.push(("avid".into(), aid.to_string())),
            VideoId::ShortUrl(_) => {
                return Err(AppError::new(AppErrorCode::E003, "Unresolved short link").into())
            }
        }
        let query = sign_query(&parameters, key, timestamp);
        let url = format!("https://api.bilibili.com/x/player/wbi/playurl?{query}");
        let response = self
            .authenticated_get(url, auth)
            .send()
            .await
            .map_err(map_reqwest_error)?;
        let envelope: ApiResponse<PlayData> = self.decode(response).await?;
        if envelope.code == -403 {
            return Err(PortError::Signature);
        }
        if envelope.code != 0 {
            return Err(PortError::Failure(map_api_error(
                envelope.code,
                envelope.message,
            )));
        }
        envelope
            .data
            .ok_or_else(|| AppError::internal("Bilibili media capabilities are missing").into())
    }
}

#[cfg(test)]
mod tests {
    use reqwest::header::COOKIE;

    use super::{
        append_bounded_chunk, map_api_error, redirect_destination, BilibiliClient, BilibiliPort,
        MAX_RESPONSE_BYTES,
    };
    use crate::infrastructure::bilibili::{
        adapt_parse_result, select_part, Clock, SystemClock, VideoId,
    };
    use crate::services::auth::{context::ValidatedAuthContext, ports::StoredCredential};

    #[test]
    fn attaches_credentials_only_to_authenticated_requests() {
        let client = BilibiliClient::new().unwrap();
        let credential =
            StoredCredential::try_new("DedeUserID=9001; SESSDATA=fixture-session".into(), None)
                .unwrap();
        let authenticated = ValidatedAuthContext::authenticated(credential);
        let anonymous = ValidatedAuthContext::anonymous();

        let authenticated_request = client
            .authenticated_get(
                "https://api.bilibili.com/x/web-interface/view",
                &authenticated,
            )
            .build()
            .unwrap();
        let anonymous_request = client
            .authenticated_get("https://api.bilibili.com/x/web-interface/view", &anonymous)
            .build()
            .unwrap();

        assert_eq!(
            authenticated_request.headers().get(COOKIE).unwrap(),
            "DedeUserID=9001; SESSDATA=fixture-session"
        );
        assert!(anonymous_request.headers().get(COOKIE).is_none());
    }

    #[test]
    fn refuses_a_chunk_before_the_response_body_exceeds_the_limit() {
        let mut body = vec![0; MAX_RESPONSE_BYTES as usize];
        let error = append_bounded_chunk(&mut body, &[1]).expect_err("limit must be enforced");
        assert_eq!(serde_json::to_value(error).unwrap()["code"], "E001");
        assert_eq!(body.len(), MAX_RESPONSE_BYTES as usize);
    }

    #[test]
    fn rejects_an_unsafe_redirect_and_the_sixth_redirect() {
        let current = url::Url::parse("https://b23.tv/fixture").unwrap();
        assert!(
            redirect_destination(&current, "https://example.com/video/BV1xx411c7BF", 0,).is_err()
        );

        let error =
            redirect_destination(&current, "https://www.bilibili.com/video/BV1xx411c7BF", 5)
                .expect_err("a sixth redirect must fail");
        assert_eq!(serde_json::to_value(error).unwrap()["code"], "E003");
    }

    #[test]
    fn maps_login_permission_and_missing_api_errors() {
        assert_eq!(
            serde_json::to_value(map_api_error(-101, String::new())).unwrap()["code"],
            "E005"
        );
        assert_eq!(
            serde_json::to_value(map_api_error(-10403, String::new())).unwrap()["code"],
            "E006"
        );
        assert_eq!(
            serde_json::to_value(map_api_error(-404, String::new())).unwrap()["code"],
            "E004"
        );
    }

    #[test]
    #[ignore = "requires opt-in access to Bilibili public endpoints"]
    fn public_anonymous_parse_smoke() {
        tauri::async_runtime::block_on(async {
            let client = BilibiliClient::new().expect("client should initialize");
            let video_id = VideoId::Bvid("BV1FNb366EH2".into());
            let auth = ValidatedAuthContext::anonymous();
            let view = client
                .fetch_view(&video_id, &auth)
                .await
                .expect("metadata should load");
            let part = select_part(&view, None).expect("video should expose a part");
            let key = client
                .fetch_wbi_key(&auth)
                .await
                .expect("WBI key should load");
            let play = client
                .fetch_playurl(&video_id, part.cid, &key, SystemClock.now_seconds(), &auth)
                .await
                .expect("media capabilities should load");
            let result = adapt_parse_result(view, play, None).expect("response should adapt");
            assert!(result
                .qualities
                .iter()
                .any(|quality| !quality.requires_login));
        });
    }
}
