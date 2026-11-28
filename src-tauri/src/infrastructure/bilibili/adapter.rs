use std::collections::HashSet;

use crate::models::{
    AppError, AppErrorCode, AudioCapability, AudioFormat, Availability, ContainerKind, MediaOption,
    ParseContainer, ParseVideoResult, UnavailableReason, VideoCodec, VideoPart, VideoSource,
    VideoVariant,
};

use super::raw::{
    PlayData, RawEpisode, RawPage, RawSeasonEpisode, RawUgcSeason, SeasonData, ViewData,
};

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

pub(crate) struct PlayCapabilities {
    pub qualities: Vec<MediaOption>,
    pub codecs: Vec<VideoCodec>,
    pub video_variants: Vec<VideoVariant>,
    pub audio_formats: Vec<AudioFormat>,
    pub audio_bitrates: Vec<MediaOption>,
    pub audio_capability: AudioCapability,
}

pub(crate) fn capabilities_from_play(play: &PlayData) -> PlayCapabilities {
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
        vec![AudioFormat::M4a, AudioFormat::Mp3, AudioFormat::Wav]
    };
    if lossless_available {
        audio_formats.push(AudioFormat::Flac);
    }

    PlayCapabilities {
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
    }
}

pub(crate) fn adapt_parse_result(
    view: ViewData,
    play: PlayData,
    requested_page: Option<u32>,
) -> Result<ParseVideoResult, AppError> {
    select_part(&view, requested_page)?;
    play.ensure_playable()?;
    let capabilities = capabilities_from_play(&play);

    Ok(ParseVideoResult {
        canonical_url: format!("https://www.bilibili.com/video/{}", view.bvid),
        bvid: Some(view.bvid.clone()),
        aid: Some(view.aid),
        source: VideoSource::Ugc,
        container: Some(ParseContainer {
            kind: ContainerKind::Video,
            id: view.bvid.clone(),
            title: view.title.clone(),
            unit_count: u32::try_from(view.pages.len()).unwrap_or(u32::MAX),
        }),
        title: view.title,
        owner_name: view.owner.name,
        cover_url: view.pic,
        duration_seconds: view.duration,
        requested_page,
        requested_cid: None,
        parts: {
            let unit_bvid = view.bvid.clone();
            let part_count = u32::try_from(view.pages.len()).unwrap_or(u32::MAX);
            view.pages
                .into_iter()
                .map(|part| VideoPart {
                    cid: part.cid,
                    bvid: unit_bvid.clone(),
                    ep_id: None,
                    page: part.page,
                    part_count,
                    group_index: 1,
                    title: part.part,
                    duration_seconds: part.duration,
                    availability: Availability::Playable,
                    unavailable_reason: None::<UnavailableReason>,
                })
                .collect()
        },
        qualities: capabilities.qualities,
        codecs: capabilities.codecs,
        video_variants: capabilities.video_variants,
        audio_formats: capabilities.audio_formats,
        audio_bitrates: capabilities.audio_bitrates,
        audio_capability: capabilities.audio_capability,
    })
}

/// Milliseconds in the PGC payload, seconds everywhere else.
pub(crate) fn duration_ms_to_seconds(value: u64) -> u64 {
    value.saturating_add(500) / 1000
}

fn collection_pages(episode: &RawSeasonEpisode) -> Vec<RawPage> {
    if !episode.pages.is_empty() {
        return episode.pages.clone();
    }
    vec![RawPage {
        cid: episode.cid,
        page: 1,
        part: episode.title.clone(),
        duration: episode.arc.as_ref().map_or(0, |arc| arc.duration),
    }]
}

pub(crate) fn adapt_collection_units(season: &RawUgcSeason) -> Vec<VideoPart> {
    let mut units = Vec::new();
    let mut seen = HashSet::new();
    let mut group_index = 0u32;
    for section in &season.sections {
        for episode in &section.episodes {
            if episode.cid == 0 && episode.pages.is_empty() {
                continue;
            }
            group_index += 1;
            let pages = collection_pages(episode);
            let part_count = u32::try_from(pages.len()).unwrap_or(u32::MAX);
            for page in pages {
                if page.cid == 0 || !seen.insert(page.cid) {
                    continue;
                }
                units.push(VideoPart {
                    cid: page.cid,
                    bvid: episode.bvid.clone(),
                    ep_id: None,
                    page: page.page.max(1),
                    part_count,
                    group_index,
                    title: page.part,
                    duration_seconds: page.duration,
                    availability: Availability::Playable,
                    unavailable_reason: None,
                });
            }
        }
    }
    units
}

fn episode_title(episode: &RawEpisode) -> String {
    if episode.long_title.trim().is_empty() {
        episode.title.clone()
    } else {
        episode.long_title.clone()
    }
}

pub(crate) fn bangumi_episode_availability(
    episode: &RawEpisode,
) -> (Availability, Option<UnavailableReason>) {
    let allow_download = episode
        .rights
        .as_ref()
        .map_or(1, |rights| rights.allow_download);
    if allow_download == 0 {
        return (
            Availability::Unavailable,
            Some(UnavailableReason::DownloadDisabled),
        );
    }
    if episode.badge.contains("大会员") || episode.badge.contains("会员") {
        return (
            Availability::Unavailable,
            Some(UnavailableReason::VipRequired),
        );
    }
    if episode.badge.contains("付费") {
        return (Availability::Unavailable, Some(UnavailableReason::Paid));
    }
    (Availability::Playable, None)
}

pub(crate) fn adapt_bangumi_units(season: &SeasonData) -> Vec<VideoPart> {
    let mut units = Vec::new();
    let mut seen = HashSet::new();
    let mut group_index = 0u32;
    for episode in &season.episodes {
        if episode.section_type != 0 || episode.cid == 0 || !seen.insert(episode.cid) {
            continue;
        }
        group_index += 1;
        let (availability, unavailable_reason) = bangumi_episode_availability(episode);
        units.push(VideoPart {
            cid: episode.cid,
            bvid: episode.bvid.clone(),
            ep_id: Some(episode.ep_id),
            page: 1,
            part_count: 1,
            group_index,
            title: episode_title(episode),
            duration_seconds: duration_ms_to_seconds(episode.duration),
            availability,
            unavailable_reason,
        });
    }
    units
}

pub(crate) fn adapt_collection_result(
    view: ViewData,
    play: PlayData,
    requested_page: Option<u32>,
) -> Result<ParseVideoResult, AppError> {
    let season = view
        .ugc_season
        .clone()
        .ok_or_else(|| unavailable("The collection metadata is missing"))?;
    // The pasted video is the entry point of the collection, so it should stay selected.
    let requested_cid = view
        .pages
        .iter()
        .find(|part| Some(part.page) == requested_page)
        .or_else(|| view.pages.first())
        .map(|part| part.cid);
    let mut result = adapt_parse_result(view, play, requested_page)?;
    let units = adapt_collection_units(&season);
    if units.is_empty() {
        return Err(unavailable("The collection has no available videos"));
    }
    if !season.title.is_empty() {
        result.title = season.title;
    }
    if !season.cover.is_empty() {
        result.cover_url = season.cover;
    }
    result.container = Some(ParseContainer {
        kind: ContainerKind::Season,
        id: season.id.to_string(),
        title: result.title.clone(),
        unit_count: u32::try_from(units.len()).unwrap_or(u32::MAX),
    });
    result.parts = units;
    result.requested_cid = requested_cid;
    Ok(result)
}

pub(crate) fn adapt_bangumi_result(
    season: SeasonData,
    requested_ep: Option<u64>,
    play: Option<PlayData>,
) -> Result<ParseVideoResult, AppError> {
    let units = adapt_bangumi_units(&season);
    if units.is_empty() {
        return Err(unavailable("The season has no playable episodes"));
    }
    let capabilities = play
        .map(|play| capabilities_from_play(&play))
        .unwrap_or(PlayCapabilities {
            qualities: Vec::new(),
            codecs: Vec::new(),
            video_variants: Vec::new(),
            audio_formats: Vec::new(),
            audio_bitrates: Vec::new(),
            audio_capability: AudioCapability {
                max_lossy_kbps: None,
                lossless_available: false,
                hi_res_available: false,
            },
        });
    let matched = requested_ep.and_then(|ep_id| {
        season
            .episodes
            .iter()
            .find(|episode| episode.ep_id == ep_id)
    });
    let (canonical_url, bvid, aid, duration_seconds) = match (requested_ep, matched) {
        (Some(ep_id), Some(episode)) => (
            format!("https://www.bilibili.com/bangumi/play/ep{ep_id}"),
            (!episode.bvid.is_empty()).then(|| episode.bvid.clone()),
            (episode.aid > 0).then_some(episode.aid),
            duration_ms_to_seconds(episode.duration),
        ),
        _ => (
            format!(
                "https://www.bilibili.com/bangumi/play/ss{}",
                season.season_id
            ),
            None,
            None,
            0,
        ),
    };
    let requested_cid = requested_ep.and_then(|ep_id| {
        units
            .iter()
            .find(|unit| unit.ep_id == Some(ep_id))
            .map(|unit| unit.cid)
    });
    Ok(ParseVideoResult {
        canonical_url,
        bvid,
        aid,
        source: VideoSource::Bangumi,
        container: Some(ParseContainer {
            kind: ContainerKind::Season,
            id: season.season_id.to_string(),
            title: season.title.clone(),
            unit_count: u32::try_from(units.len()).unwrap_or(u32::MAX),
        }),
        title: season.title,
        owner_name: String::new(),
        cover_url: season.cover,
        duration_seconds,
        requested_page: None,
        requested_cid,
        parts: units,
        qualities: capabilities.qualities,
        codecs: capabilities.codecs,
        video_variants: capabilities.video_variants,
        audio_formats: capabilities.audio_formats,
        audio_bitrates: capabilities.audio_bitrates,
        audio_capability: capabilities.audio_capability,
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
    use super::duration_ms_to_seconds;
    use crate::infrastructure::bilibili::raw::{ApiResponse, PlayData, ViewData};
    use crate::models::{
        AudioFormat, Availability, ContainerKind, VideoCodec, VideoSource, VideoVariant,
    };

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

        assert_eq!(result.bvid.as_deref(), Some("BV1xx411c7BF"));
        assert_eq!(result.requested_page, Some(2));
        assert_eq!(result.requested_cid, None);
        assert_eq!(result.parts.len(), 2);
        assert_eq!(result.source, VideoSource::Ugc);
        let container = result.container.expect("container should be present");
        assert_eq!(container.kind, ContainerKind::Video);
        assert_eq!(container.unit_count, 2);
        assert_eq!(result.parts[0].bvid, "BV1xx411c7BF");
        assert_eq!(result.parts[0].part_count, 2);
        assert_eq!(result.parts[0].group_index, 1);
        assert_eq!(result.parts[0].availability, Availability::Playable);
        assert_eq!(result.parts[0].unavailable_reason, None);
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
            vec![AudioFormat::M4a, AudioFormat::Mp3, AudioFormat::Wav]
        );
        assert_eq!(result.audio_bitrates[0].id, "192");
    }

    #[test]
    fn converts_pgc_milliseconds_to_rounded_seconds() {
        assert_eq!(duration_ms_to_seconds(0), 0);
        assert_eq!(duration_ms_to_seconds(499), 0);
        assert_eq!(duration_ms_to_seconds(500), 1);
        assert_eq!(duration_ms_to_seconds(1_203_160), 1203);
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
    fn rejects_voucher_and_empty_dash_payloads_before_exposing_parse_success() {
        let view: ApiResponse<ViewData> =
            serde_json::from_str(include_str!("../../../tests/fixtures/bilibili-view.json"))
                .unwrap();
        let payloads = [
            r#"{
              "code":0,"message":"0","data":{
                "accept_quality":[80],"accept_description":["1080P"],
                "v_voucher":"fixture-voucher"
              }
            }"#,
            r#"{
              "code":0,"message":"0","data":{
                "accept_quality":[80],"accept_description":["1080P"],
                "dash":{}
              }
            }"#,
        ];

        for payload in payloads {
            let play: ApiResponse<PlayData> = serde_json::from_str(payload).unwrap();
            let error = adapt_parse_result(view.data.clone().unwrap(), play.data.unwrap(), None)
                .expect_err("unplayable WBI payloads must not become successful parse results");

            assert_eq!(error.code, crate::models::AppErrorCode::E004);
        }
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
