use crate::models::{
    AppError, AppErrorCode, AudioCapability, AudioFormat, MediaOption, ParseVideoResult,
    VideoCodec, VideoPart, VideoVariant,
};

use super::raw::{PlayData, RawPage, ViewData};

fn unavailable(message: impl Into<String>) -> AppError {
    AppError::new(AppErrorCode::E004, message)
}

pub(crate) fn select_part(
    view: &ViewData,
    requested_page: Option<u32>,
) -> Result<RawPage, AppError> {
    let selected = match requested_page {
        Some(page) => view.pages.iter().find(|item| item.page == page),
        None => view.pages.first(),
    };
    selected
        .cloned()
        .ok_or_else(|| unavailable("The requested video part is unavailable"))
}

pub(crate) fn adapt_parse_result(
    view: ViewData,
    play: PlayData,
    requested_page: Option<u32>,
) -> Result<ParseVideoResult, AppError> {
    select_part(&view, requested_page)?;

    let video_variants = play
        .dash
        .as_ref()
        .map(|dash| {
            play.accept_quality
                .iter()
                .flat_map(|quality| {
                    [VideoCodec::Avc, VideoCodec::Hevc, VideoCodec::Av1]
                        .into_iter()
                        .filter_map(|codec| {
                            dash.video
                                .iter()
                                .any(|stream| {
                                    stream.id == *quality
                                        && codec_for(&stream.codecs) == Some(codec.clone())
                                })
                                .then(|| VideoVariant {
                                    quality_id: quality.to_string(),
                                    codec,
                                })
                        })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let qualities = play
        .accept_quality
        .iter()
        .enumerate()
        .map(|(index, quality)| MediaOption {
            id: quality.to_string(),
            label: play
                .accept_description
                .get(index)
                .cloned()
                .unwrap_or_else(|| format!("Quality {quality}")),
            requires_login: !video_variants
                .iter()
                .any(|variant| variant.quality_id == quality.to_string()),
        })
        .collect::<Vec<_>>();

    let mut codecs = Vec::new();
    let mut bitrate_values = Vec::new();
    if let Some(dash) = &play.dash {
        for codec in [VideoCodec::Avc, VideoCodec::Hevc, VideoCodec::Av1] {
            if video_variants.iter().any(|variant| variant.codec == codec) {
                codecs.push(codec);
            }
        }
        for stream in &dash.audio {
            let kbps = stream.bandwidth / 1000;
            if kbps > 0 && !bitrate_values.contains(&kbps) {
                bitrate_values.push(kbps);
            }
        }
    }
    bitrate_values.sort_unstable_by(|left, right| right.cmp(left));
    let max_lossy_kbps = bitrate_values.iter().copied().max();
    let audio_bitrates = bitrate_values
        .into_iter()
        .map(|kbps| MediaOption {
            id: kbps.to_string(),
            label: format!("{kbps} kbps"),
            requires_login: false,
        })
        .collect::<Vec<_>>();
    let lossless_available = play
        .dash
        .as_ref()
        .and_then(|dash| dash.flac.as_ref())
        .is_some_and(|flac| flac.display && flac.audio.is_some());
    let mut audio_formats = if audio_bitrates.is_empty() {
        Vec::new()
    } else {
        vec![AudioFormat::M4a, AudioFormat::Mp3]
    };
    if lossless_available {
        audio_formats.push(AudioFormat::Flac);
    }

    Ok(ParseVideoResult {
        canonical_url: format!("https://www.bilibili.com/video/{}", view.bvid),
        bvid: view.bvid,
        aid: view.aid,
        title: view.title,
        owner_name: view.owner.name,
        cover_url: view.pic,
        duration_seconds: view.duration,
        requested_page,
        parts: view
            .pages
            .into_iter()
            .map(|part| VideoPart {
                cid: part.cid,
                page: part.page,
                title: part.part,
                duration_seconds: part.duration,
            })
            .collect(),
        qualities,
        codecs,
        video_variants,
        audio_formats,
        audio_bitrates,
        audio_capability: AudioCapability {
            max_lossy_kbps,
            lossless_available,
            hi_res_available: lossless_available,
        },
    })
}

fn codec_for(value: &str) -> Option<VideoCodec> {
    if value.starts_with("av01") {
        Some(VideoCodec::Av1)
    } else if value.starts_with("hev") || value.starts_with("hvc") {
        Some(VideoCodec::Hevc)
    } else if value.starts_with("avc") {
        Some(VideoCodec::Avc)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::adapt_parse_result;
    use crate::infrastructure::bilibili::raw::{ApiResponse, PlayData, ViewData};
    use crate::models::{AudioFormat, VideoCodec, VideoVariant};

    #[test]
    fn maps_private_api_fixtures_to_the_stable_contract() {
        let view: ApiResponse<ViewData> =
            serde_json::from_str(include_str!("../../../tests/fixtures/bilibili-view.json"))
                .expect("view fixture should parse");
        let play: ApiResponse<PlayData> = serde_json::from_str(include_str!(
            "../../../tests/fixtures/bilibili-playurl.json"
        ))
        .expect("play fixture should parse");

        let result = adapt_parse_result(view.data.unwrap(), play.data.unwrap(), Some(2))
            .expect("fixtures should adapt");

        assert_eq!(result.bvid, "BV1xx411c7BF");
        assert_eq!(result.requested_page, Some(2));
        assert_eq!(result.parts.len(), 2);
        assert_eq!(result.qualities[0].id, "120");
        assert!(result.qualities[0].requires_login);
        assert!(!result.qualities[1].requires_login);
        assert_eq!(
            result.codecs,
            vec![VideoCodec::Avc, VideoCodec::Hevc, VideoCodec::Av1]
        );
        assert_eq!(
            result.video_variants,
            vec![
                VideoVariant {
                    quality_id: "80".into(),
                    codec: VideoCodec::Avc
                },
                VideoVariant {
                    quality_id: "80".into(),
                    codec: VideoCodec::Hevc
                },
                VideoVariant {
                    quality_id: "80".into(),
                    codec: VideoCodec::Av1
                },
                VideoVariant {
                    quality_id: "64".into(),
                    codec: VideoCodec::Avc
                },
                VideoVariant {
                    quality_id: "32".into(),
                    codec: VideoCodec::Avc
                },
            ]
        );
        assert_eq!(
            result.audio_formats,
            vec![AudioFormat::M4a, AudioFormat::Mp3]
        );
        assert_eq!(result.audio_bitrates[0].id, "192");
    }

    #[test]
    fn rejects_a_requested_part_that_is_not_present() {
        let view: ApiResponse<ViewData> =
            serde_json::from_str(include_str!("../../../tests/fixtures/bilibili-view.json"))
                .unwrap();
        let play: ApiResponse<PlayData> = serde_json::from_str(include_str!(
            "../../../tests/fixtures/bilibili-playurl.json"
        ))
        .unwrap();

        let error =
            adapt_parse_result(view.data.unwrap(), play.data.unwrap(), Some(99)).unwrap_err();
        assert_eq!(serde_json::to_value(error).unwrap()["code"], "E004");
    }

    #[test]
    fn exposes_lossless_capability_only_when_dash_contains_a_lossless_stream() {
        let view: ApiResponse<ViewData> =
            serde_json::from_str(include_str!("../../../tests/fixtures/bilibili-view.json"))
                .unwrap();
        let play: ApiResponse<PlayData> = serde_json::from_str(
            r#"{
              "code":0,"message":"0","data":{
                "accept_quality":[],"accept_description":[],
                "dash":{"video":[],"audio":[],"flac":{"display":true,"audio":{
                  "id":30251,"baseUrl":"https://a.bilivideo.com/lossless.m4s",
                  "backupUrl":[],"bandwidth":1411000,"mimeType":"audio/flac","codecs":"fLaC"
                }}}
              }
            }"#,
        )
        .unwrap();

        let result = adapt_parse_result(view.data.unwrap(), play.data.unwrap(), None).unwrap();

        assert!(result.audio_capability.lossless_available);
        assert!(result.audio_capability.hi_res_available);
        assert!(result.audio_formats.contains(&AudioFormat::Flac));
    }
}
