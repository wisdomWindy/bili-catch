use std::sync::Arc;

use async_trait::async_trait;
use reqwest::header::{COOKIE, USER_AGENT};
use serde::Deserialize;
use url::Url;

use crate::{
    models::{AppError, AppErrorCode, TranscriptSegment},
    services::{
        auth::{context::AuthContextProvider, AuthManager},
        transcription::ports::{SubtitleLookup, SubtitleProvider, TranscriberOutput},
    },
};

pub struct BilibiliSubtitleProvider {
    auth: Arc<AuthManager>,
    client: reqwest::Client,
}

impl BilibiliSubtitleProvider {
    pub(crate) fn new(auth: Arc<AuthManager>) -> Result<Self, AppError> {
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::limited(3))
            .build()
            .map_err(|_| subtitle_error("SUBTITLE_CLIENT_UNAVAILABLE"))?;
        Ok(Self { auth, client })
    }
}

#[async_trait]
impl SubtitleProvider for BilibiliSubtitleProvider {
    async fn lookup(&self, bvid: &str, cid: u64) -> SubtitleLookup {
        let auth = match self.auth.validated_context().await {
            Ok(value) => value,
            Err(error) => return SubtitleLookup::Failed(error),
        };
        if !auth.is_authenticated() {
            return SubtitleLookup::Unavailable;
        }
        let Some(credential) = auth.credential() else {
            return SubtitleLookup::Unavailable;
        };
        let response = self
            .client
            .get("https://api.bilibili.com/x/player/v2")
            .query(&[("bvid", bvid.to_owned()), ("cid", cid.to_string())])
            .header(USER_AGENT, "Mozilla/5.0 BiliCatch/0.1")
            .header(COOKIE, credential.expose_cookie_header())
            .send()
            .await;
        let response = match response {
            Ok(value) if value.status().is_success() => value,
            Ok(_) => return SubtitleLookup::Failed(subtitle_error("SUBTITLE_INDEX_HTTP")),
            Err(_) => return SubtitleLookup::Failed(subtitle_error("SUBTITLE_INDEX_NETWORK")),
        };
        let index = match response.json::<PlayerEnvelope>().await {
            Ok(value) if value.code == 0 => value,
            Ok(_) => return SubtitleLookup::Failed(subtitle_error("SUBTITLE_INDEX_REJECTED")),
            Err(_) => return SubtitleLookup::Failed(subtitle_error("SUBTITLE_INDEX_INVALID")),
        };
        let Some(candidate) = index
            .data
            .and_then(|data| data.subtitle)
            .and_then(|subtitle| subtitle.subtitles.into_iter().next())
        else {
            return SubtitleLookup::Unavailable;
        };
        let url = match validated_subtitle_url(&candidate.subtitle_url) {
            Ok(value) => value,
            Err(error) => return SubtitleLookup::Failed(error),
        };
        let response = match self.client.get(url).send().await {
            Ok(value) if value.status().is_success() => value,
            Ok(_) => return SubtitleLookup::Failed(subtitle_error("SUBTITLE_BODY_HTTP")),
            Err(_) => return SubtitleLookup::Failed(subtitle_error("SUBTITLE_BODY_NETWORK")),
        };
        let body = match response.bytes().await {
            Ok(value) => value,
            Err(_) => return SubtitleLookup::Failed(subtitle_error("SUBTITLE_BODY_NETWORK")),
        };
        match parse_subtitle_body(&body, &candidate.language) {
            Ok(Some(output)) => SubtitleLookup::Available(output),
            Ok(None) => SubtitleLookup::Unavailable,
            Err(error) => SubtitleLookup::Failed(error),
        }
    }
}

fn validated_subtitle_url(raw: &str) -> Result<Url, AppError> {
    let value = if raw.starts_with("//") {
        format!("https:{raw}")
    } else {
        raw.to_owned()
    };
    let url = Url::parse(&value).map_err(|_| subtitle_error("SUBTITLE_URL_INVALID"))?;
    let trusted = url.scheme() == "https"
        && url.host_str().is_some_and(|host| {
            host == "bilibili.com"
                || host.ends_with(".bilibili.com")
                || host == "hdslb.com"
                || host.ends_with(".hdslb.com")
        });
    if !trusted {
        return Err(subtitle_error("SUBTITLE_URL_UNTRUSTED"));
    }
    Ok(url)
}

fn parse_subtitle_body(body: &[u8], language: &str) -> Result<Option<TranscriberOutput>, AppError> {
    let document: SubtitleDocument =
        serde_json::from_slice(body).map_err(|_| subtitle_error("SUBTITLE_BODY_INVALID"))?;
    let segments = document
        .body
        .into_iter()
        .enumerate()
        .filter_map(|(index, item)| {
            let text = item.content.trim().to_owned();
            if text.is_empty() || !item.from.is_finite() || !item.to.is_finite() {
                return None;
            }
            let start_ms = (item.from.max(0.0) * 1000.0).round() as u64;
            let end_ms = (item.to.max(item.from).max(0.0) * 1000.0).round() as u64;
            Some(TranscriptSegment {
                index: index as u32,
                start_ms,
                end_ms,
                text,
            })
        })
        .collect::<Vec<_>>();
    if segments.is_empty() {
        return Ok(None);
    }
    Ok(Some(TranscriberOutput {
        detected_language: language.trim().to_owned(),
        segments,
    }))
}

fn subtitle_error(details: &'static str) -> AppError {
    AppError {
        code: AppErrorCode::E014,
        message: "Platform subtitles are unavailable".into(),
        details: Some(details.into()),
    }
}

#[derive(Deserialize)]
struct PlayerEnvelope {
    code: i64,
    data: Option<PlayerData>,
}

#[derive(Deserialize)]
struct PlayerData {
    subtitle: Option<SubtitleIndex>,
}

#[derive(Deserialize)]
struct SubtitleIndex {
    #[serde(default)]
    subtitles: Vec<SubtitleCandidate>,
}

#[derive(Deserialize)]
struct SubtitleCandidate {
    #[serde(rename = "lan")]
    language: String,
    subtitle_url: String,
}

#[derive(Deserialize)]
struct SubtitleDocument {
    #[serde(default)]
    body: Vec<SubtitleLine>,
}

#[derive(Deserialize)]
struct SubtitleLine {
    from: f64,
    to: f64,
    content: String,
}

#[cfg(test)]
mod tests {
    use super::{parse_subtitle_body, validated_subtitle_url};

    #[test]
    fn parses_platform_subtitles_into_normalized_segments() {
        let output = parse_subtitle_body(
            br#"{"body":[{"from":0.25,"to":1.5,"content":" hello "},{"from":1.5,"to":2.0,"content":"world"}]}"#,
            "en",
        )
        .unwrap()
        .unwrap();

        assert_eq!(output.detected_language, "en");
        assert_eq!(output.segments[0].start_ms, 250);
        assert_eq!(output.segments[0].end_ms, 1500);
        assert_eq!(output.segments[0].text, "hello");
        assert_eq!(output.segments[1].index, 1);
    }

    #[test]
    fn accepts_only_https_bilibili_subtitle_hosts() {
        assert!(validated_subtitle_url("//i0.hdslb.com/bfs/subtitle/demo.json").is_ok());
        assert!(validated_subtitle_url("https://api.bilibili.com/subtitle.json").is_ok());
        assert!(validated_subtitle_url("http://i0.hdslb.com/demo.json").is_err());
        assert!(validated_subtitle_url("https://hdslb.com.example.org/demo.json").is_err());
    }
}
