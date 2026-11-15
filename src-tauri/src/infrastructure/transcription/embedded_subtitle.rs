use crate::models::{TranscriptOrigin, TranscriptSegment};

/// 解析 SRT 字幕为转写分段。
///
/// 输入来自 `ffmpeg -f srt -` 的 stdout；解析失败或为空时返回空向量，
/// 由调用方降级到下一级信源，而不是让整个转写任务失败。
pub fn parse_srt(bytes: &[u8]) -> Vec<TranscriptSegment> {
    let text = String::from_utf8_lossy(bytes);
    let text = text.trim_start_matches('\u{feff}').replace("\r\n", "\n");
    let mut segments = Vec::new();
    for block in text.split("\n\n") {
        let block = block.trim();
        if block.is_empty() {
            continue;
        }
        let mut lines = block.lines();
        let Some((start_ms, end_ms, body)) = split_block(lines.by_ref()) else {
            continue;
        };
        let content = body
            .iter()
            .map(|line| clean_cue_line(line))
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join(" ");
        if content.is_empty() {
            continue;
        }
        segments.push(TranscriptSegment {
            index: segments.len() as u32,
            start_ms,
            end_ms,
            text: content,
            confidence: Some(1.0),
            origin: TranscriptOrigin::EmbeddedSubtitle,
            ..TranscriptSegment::default()
        });
    }
    segments
}

fn split_block<'a, I>(lines: I) -> Option<(u64, u64, Vec<&'a str>)>
where
    I: Iterator<Item = &'a str>,
{
    let lines = lines.collect::<Vec<_>>();
    let cue = lines.iter().position(|line| line.contains("-->"))?;
    let (start, end) = parse_cue_times(lines[cue])?;
    let body = lines[cue + 1..].to_vec();
    Some((start, end, body))
}

fn parse_cue_times(line: &str) -> Option<(u64, u64)> {
    let (left, right) = line.split_once("-->")?;
    let start = parse_srt_timestamp(left)?;
    let end = parse_srt_timestamp(right)?;
    Some((start, end.max(start)))
}

fn parse_srt_timestamp(value: &str) -> Option<u64> {
    let value = value.trim();
    let mut parts = value.split(':');
    let hours = parts.next()?.parse::<u64>().ok()?;
    let minutes = parts.next()?.parse::<u64>().ok()?;
    let seconds_part = parts.next()?;
    if parts.next().is_some() {
        return None;
    }
    let (seconds, millis) = match seconds_part.split_once([',', '.']) {
        Some((seconds, millis)) => (seconds, millis),
        None => (seconds_part, "0"),
    };
    let seconds = seconds.parse::<u64>().ok()?;
    let millis = format!("{millis:0<3}");
    let millis = millis.get(..3)?.parse::<u64>().ok()?;
    Some(((hours * 60 + minutes) * 60 + seconds) * 1000 + millis)
}

fn clean_cue_line(line: &str) -> String {
    let mut output = String::with_capacity(line.len());
    let mut tag_depth = 0_u32;
    for character in line.chars() {
        match character {
            '{' => tag_depth += 1,
            '}' => tag_depth = tag_depth.saturating_sub(1),
            '<' => tag_depth += 1,
            '>' => tag_depth = tag_depth.saturating_sub(1),
            _ if tag_depth == 0 => output.push(character),
            _ => {}
        }
    }
    output.trim().to_owned()
}

#[cfg(test)]
mod tests {
    use super::parse_srt;
    use crate::models::TranscriptOrigin;

    #[test]
    fn parses_bom_crlf_and_multi_line_cues() {
        let srt = "\u{feff}1\r\n00:00:01,250 --> 00:00:02,500\r\nHello\r\nthere\r\n\r\n2\r\n00:00:03,000 --> 00:00:04,000\r\n第二句\r\n";

        let segments = parse_srt(srt.as_bytes());

        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].start_ms, 1_250);
        assert_eq!(segments[0].end_ms, 2_500);
        assert_eq!(segments[0].text, "Hello there");
        assert_eq!(segments[1].text, "第二句");
        assert_eq!(segments[0].origin, TranscriptOrigin::EmbeddedSubtitle);
        assert_eq!(segments[0].confidence, Some(1.0));
    }

    #[test]
    fn strips_ass_and_html_markup() {
        let srt = "1\n00:00:00,500 --> 00:00:01,000\n{\\an8}<i>加粗</i>文本\n";

        let segments = parse_srt(srt.as_bytes());

        assert_eq!(segments[0].text, "加粗文本");
    }

    #[test]
    fn ignores_blocks_without_a_valid_cue() {
        let srt = "not a subtitle\n\n1\nbad --> times\ntext\n";

        assert!(parse_srt(srt.as_bytes()).is_empty());
    }
}
