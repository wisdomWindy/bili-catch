use crate::models::{SegmentAlternate, TranscriptOrigin, TranscriptSegment};

/// 置信度高于此值时认为两个信源的文本一致。
const MATCH_SIMILARITY: f32 = 0.85;
const CONFLICT_CONFIDENCE: f32 = 0.4;
const CONFLICT_RATIO_LIMIT: f32 = 0.15;

#[derive(Debug, Clone, PartialEq)]
pub struct Candidate {
    pub origin: TranscriptOrigin,
    pub segments: Vec<TranscriptSegment>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct FusionStats {
    pub sources_used: Vec<TranscriptOrigin>,
    pub conflict_count: u32,
    pub low_confidence_count: u32,
    pub review_recommended: bool,
}

/// 信源优先级：字幕轨 > 平台字幕 > OCRO > ASR。
fn priority(origin: TranscriptOrigin) -> u8 {
    match origin {
        TranscriptOrigin::EmbeddedSubtitle => 0,
        TranscriptOrigin::PlatformSubtitle => 1,
        TranscriptOrigin::Fused => 2,
        TranscriptOrigin::HardSubtitleOcr => 3,
        TranscriptOrigin::Asr => 4,
    }
}

/// 合并多个信源的候选段落。
///
/// 以优先级最高的候选为骨架，其余候选只用于补足空白时间段；
/// 时间重叠但文本不一致的段落保留备选文本并降低置信度。
pub fn fuse(mut candidates: Vec<Candidate>) -> (Vec<TranscriptSegment>, FusionStats) {
    candidates.retain(|candidate| !candidate.segments.is_empty());
    let mut stats = FusionStats::default();
    if candidates.is_empty() {
        return (Vec::new(), stats);
    }
    candidates.sort_by_key(|candidate| priority(candidate.origin));
    stats.sources_used = candidates.iter().map(|candidate| candidate.origin).collect();

    let mut merged = candidates[0].segments.clone();
    for segment in &mut merged {
        segment.origin = if candidates.len() > 1 {
            TranscriptOrigin::Fused
        } else {
            segment.origin
        };
    }

    for candidate in candidates.iter().skip(1) {
        for segment in &candidate.segments {
            match overlapping(&merged, segment) {
                Some(index) => {
                    let similarity = similarity(&merged[index].text, &segment.text);
                    if similarity >= MATCH_SIMILARITY {
                        let confidence = merged[index].confidence.unwrap_or(0.5).max(
                            segment.confidence.unwrap_or(0.5),
                        );
                        merged[index].confidence = Some(confidence);
                    } else {
                        stats.conflict_count += 1;
                        merged[index].alternates.push(SegmentAlternate {
                            origin: segment.origin,
                            text: segment.text.clone(),
                        });
                        merged[index].confidence = Some(CONFLICT_CONFIDENCE);
                    }
                }
                None => merged.push(segment.clone()),
            }
        }
    }

    merged.sort_by_key(|segment| segment.start_ms);
    for (index, segment) in merged.iter_mut().enumerate() {
        segment.index = index as u32;
    }
    stats.low_confidence_count = merged
        .iter()
        .filter(|segment| segment.confidence.is_some_and(|value| value < 0.55))
        .count() as u32;
    if !merged.is_empty() {
        stats.review_recommended =
            stats.conflict_count as f32 / merged.len() as f32 > CONFLICT_RATIO_LIMIT;
    }
    (merged, stats)
}

fn overlapping<'a>(
    segments: &'a [TranscriptSegment],
    target: &TranscriptSegment,
) -> Option<usize> {
    segments.iter().position(|segment| {
        segment.start_ms < target.end_ms.max(target.start_ms + 1)
            && target.start_ms < segment.end_ms.max(segment.start_ms + 1)
    })
}

fn similarity(left: &str, right: &str) -> f32 {
    let left = normalize(left);
    let right = normalize(right);
    if left.is_empty() || right.is_empty() {
        return 0.0;
    }
    let longest = left.len().max(right.len()) as f32;
    let distance = levenshtein(&left, &right) as f32;
    (1.0 - distance / longest).clamp(0.0, 1.0)
}

fn normalize(text: &str) -> Vec<char> {
    text.chars()
        .filter(|character| !character.is_whitespace() && !is_punctuation(*character))
        .map(|character| match character {
            '，' => '，',
            other => other.to_ascii_lowercase(),
        })
        .collect()
}

fn is_punctuation(character: char) -> bool {
    matches!(
        character,
        ',' | '.'
            | '!'
            | '?'
            | ';'
            | ':'
            | '，'
            | '。'
            | '！'
            | '？'
            | '；'
            | '：'
            | '、'
            | '“'
            | '”'
            | '‘'
            | '’'
    )
}

fn levenshtein(left: &[char], right: &[char]) -> usize {
    let mut previous = (0..=right.len()).collect::<Vec<_>>();
    let mut current = vec![0_usize; right.len() + 1];
    for (i, left_char) in left.iter().enumerate() {
        current[0] = i + 1;
        for (j, right_char) in right.iter().enumerate() {
            let cost = usize::from(left_char != right_char);
            current[j + 1] = (previous[j + 1] + 1)
                .min(current[j] + 1)
                .min(previous[j] + cost);
        }
        std::mem::swap(&mut previous, &mut current);
    }
    previous[right.len()]
}

#[cfg(test)]
mod tests {
    use super::{fuse, similarity, Candidate};
    use crate::models::{TranscriptOrigin, TranscriptSegment};

    fn segment(index: u32, start: u64, end: u64, text: &str) -> TranscriptSegment {
        TranscriptSegment {
            index,
            start_ms: start,
            end_ms: end,
            text: text.into(),
            confidence: Some(0.9),
            ..TranscriptSegment::default()
        }
    }

    #[test]
    fn identical_sources_merge_without_conflicts() {
        let (segments, stats) = fuse(vec![
            Candidate {
                origin: TranscriptOrigin::PlatformSubtitle,
                segments: vec![segment(0, 0, 1_000, "今天我们来聊聊 Rust")],
            },
            Candidate {
                origin: TranscriptOrigin::Asr,
                segments: vec![segment(0, 0, 1_000, "今天我们来聊聊Rust。")],
            },
        ]);

        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].origin, TranscriptOrigin::Fused);
        assert_eq!(stats.conflict_count, 0);
    }

    #[test]
    fn different_text_becomes_an_alternate_with_low_confidence() {
        let (segments, stats) = fuse(vec![
            Candidate {
                origin: TranscriptOrigin::HardSubtitleOcr,
                segments: vec![segment(0, 0, 2_000, "完全不同的字幕内容")],
            },
            Candidate {
                origin: TranscriptOrigin::Asr,
                segments: vec![segment(0, 0, 2_000, "另一份识别结果")],
            },
        ]);

        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].text, "完全不同的字幕内容");
        assert_eq!(segments[0].alternates.len(), 1);
        assert_eq!(segments[0].confidence, Some(0.4));
        assert_eq!(stats.conflict_count, 1);
    }

    #[test]
    fn non_overlapping_segments_are_appended_and_resorted() {
        let (segments, _) = fuse(vec![
            Candidate {
                origin: TranscriptOrigin::PlatformSubtitle,
                segments: vec![segment(0, 5_000, 6_000, "第二段")],
            },
            Candidate {
                origin: TranscriptOrigin::Asr,
                segments: vec![segment(0, 0, 1_000, "第一段")],
            },
        ]);

        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].text, "第一段");
        assert_eq!(segments[1].index, 1);
    }

    #[test]
    fn similarity_ignores_punctuation_and_whitespace() {
        assert!(similarity("你好，世界！", "你好世界") > 0.95);
        assert!(similarity("abc", "xyz") < 0.2);
    }
}
