use crate::models::AppError;
use crate::models::AppErrorCode;
use url::Url;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VideoId {
    Bvid(String),
    Aid(u64),
    ShortUrl(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedInput {
    pub video_id: VideoId,
    pub requested_page: Option<u32>,
}

fn invalid_input() -> AppError {
    AppError {
        code: AppErrorCode::E003,
        message: "Invalid Bilibili video input".into(),
        details: None,
    }
}

fn parse_video_id(value: &str) -> Option<VideoId> {
    if value.len() == 12
        && value.starts_with("BV")
        && value.bytes().all(|byte| byte.is_ascii_alphanumeric())
    {
        return Some(VideoId::Bvid(value.to_owned()));
    }

    if value.len() > 2 && value[..2].eq_ignore_ascii_case("av") {
        return value[2..]
            .parse::<u64>()
            .ok()
            .filter(|aid| *aid > 0)
            .map(VideoId::Aid);
    }
    None
}

pub(crate) fn is_allowed_host(host: &str) -> bool {
    host == "b23.tv" || host == "bilibili.com" || host.ends_with(".bilibili.com")
}

pub(crate) fn validate_allowed_https_url(value: &str) -> Result<Url, AppError> {
    let url = Url::parse(value).map_err(|_| invalid_input())?;
    if url.scheme() != "https" {
        return Err(invalid_input());
    }
    let host = url
        .host_str()
        .map(str::to_ascii_lowercase)
        .ok_or_else(invalid_input)?;
    if !is_allowed_host(&host) {
        return Err(invalid_input());
    }
    Ok(url)
}

pub fn normalize_input(input: &str) -> Result<NormalizedInput, AppError> {
    let value = input.trim();
    if value.is_empty()
        || value
            .chars()
            .any(|character| character.is_whitespace() || character.is_control())
    {
        return Err(invalid_input());
    }

    if let Some(video_id) = parse_video_id(value) {
        return Ok(NormalizedInput {
            video_id,
            requested_page: None,
        });
    }

    let url = validate_allowed_https_url(value)?;
    let host = url
        .host_str()
        .map(str::to_ascii_lowercase)
        .ok_or_else(invalid_input)?;
    if host == "b23.tv" {
        if url.path().trim_matches('/').is_empty() {
            return Err(invalid_input());
        }
        return Ok(NormalizedInput {
            video_id: VideoId::ShortUrl(url.to_string()),
            requested_page: None,
        });
    }

    let video_id = url
        .path_segments()
        .and_then(|mut segments| segments.next_back())
        .and_then(parse_video_id)
        .ok_or_else(invalid_input)?;
    let requested_page = match url.query_pairs().find(|(key, _)| key == "p") {
        Some((_, value)) => Some(
            value
                .parse::<u32>()
                .ok()
                .filter(|page| *page > 0)
                .ok_or_else(invalid_input)?,
        ),
        None => None,
    };

    Ok(NormalizedInput {
        video_id,
        requested_page,
    })
}

#[cfg(test)]
mod tests {
    use super::{normalize_input, NormalizedInput, VideoId};

    #[test]
    fn normalizes_supported_ids_and_urls() {
        let cases = [
            (" BV1xx411c7BF ", VideoId::Bvid("BV1xx411c7BF".into()), None),
            ("av170001", VideoId::Aid(170001), None),
            ("AV170001", VideoId::Aid(170001), None),
            (
                "https://www.bilibili.com/video/BV1xx411c7BF?p=3",
                VideoId::Bvid("BV1xx411c7BF".into()),
                Some(3),
            ),
            (
                "https://m.bilibili.com/video/av170001",
                VideoId::Aid(170001),
                None,
            ),
            (
                "https://b23.tv/abcdef",
                VideoId::ShortUrl("https://b23.tv/abcdef".into()),
                None,
            ),
        ];

        for (input, video_id, requested_page) in cases {
            assert_eq!(
                normalize_input(input).expect("input should normalize"),
                NormalizedInput {
                    video_id,
                    requested_page
                }
            );
        }
    }

    #[test]
    fn rejects_blank_malformed_and_unsafe_inputs() {
        for input in [
            "",
            " \n\t ",
            "BV1 xx411c7BF",
            "BV1xx411c7BF\nhttps://example.com",
            "http://www.bilibili.com/video/BV1xx411c7BF",
            "https://bilibili.com.evil.example/video/BV1xx411c7BF",
            "https://example.com/video/BV1xx411c7BF",
            "https://www.bilibili.com/video/BV1xx411c7BF?p=0",
            "https://www.bilibili.com/video/BV1xx411c7BF?p=oops",
        ] {
            assert!(
                normalize_input(input).is_err(),
                "{input} should be rejected"
            );
        }
    }
}
