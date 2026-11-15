use crate::models::{TranscriptOrigin, TranscriptSegment};

use super::ocr_engine::{OcrEngine, OcrLine};

/// 只在下 45% 的画面里找字幕带。
const BOTTOM_RATIO: f32 = 0.45;
/// 单行文本至少占这么多行高，低于此值视为噪声。
const MIN_BAND_ROWS: usize = 6;
/// 相邻字幕行合并的最大间隔。
const MAX_BAND_GAP: usize = 3;
/// 判定「像文字」的相邻像素差阈值（0~255 灰度）。
const EDGE_DELTA: u32 = 40;
const MIN_EDGES_PER_ROW: usize = 2;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrayFrame {
    pub width: usize,
    pub height: usize,
    pub data: Vec<u8>,
}

impl GrayFrame {
    pub fn new(width: usize, height: usize, data: Vec<u8>) -> Option<Self> {
        (width > 0 && height > 0 && data.len() >= width * height).then_some(Self {
            width,
            height,
            data,
        })
    }

    fn row(&self, y: usize) -> &[u8] {
        &self.data[y * self.width..(y + 1) * self.width]
    }
}

/// 返回候选字幕带的 `[start_row, end_row)`，最多两条，按行号升序。
pub fn detect_bands(frames: &[GrayFrame]) -> Vec<(usize, usize)> {
    let Some(first) = frames.first() else {
        return Vec::new();
    };
    let region_start = (first.height as f32 * (1.0 - BOTTOM_RATIO)) as usize;
    if region_start >= first.height {
        return Vec::new();
    }

    let mut active = vec![false; first.height];
    for y in region_start..first.height {
        let hits = frames
            .iter()
            .filter(|frame| row_has_text(frame.row(y)))
            .count();
        active[y] = hits * 2 >= frames.len();
    }

    let mut bands = Vec::new();
    let mut y = region_start;
    while y < first.height {
        if !active[y] {
            y += 1;
            continue;
        }
        let start = y;
        let mut end = y;
        let mut gap = 0;
        while y < first.height {
            if active[y] {
                end = y + 1;
                gap = 0;
            } else {
                gap += 1;
                if gap > MAX_BAND_GAP {
                    break;
                }
            }
            y += 1;
        }
        if end - start >= MIN_BAND_ROWS {
            bands.push((start.saturating_sub(2), (end + 2).min(first.height)));
        }
    }
    bands.truncate(2);
    bands
}

fn row_has_text(row: &[u8]) -> bool {
    let mut edges = 0;
    for pair in row.windows(2) {
        if (pair[1] as i32 - pair[0] as i32).unsigned_abs() >= EDGE_DELTA {
            edges += 1;
            if edges >= MIN_EDGES_PER_ROW {
                return true;
            }
        }
    }
    false
}

/// 对候选字幕带逐帧识别，合并为带时间轴的句子。
pub fn read_bands(
    engine: &OcrEngine,
    frames: &[GrayFrame],
    fps: f64,
) -> Result<Vec<TranscriptSegment>, crate::models::AppError> {
    let bands = detect_bands(frames);
    if bands.is_empty() || fps <= 0.0 {
        return Ok(Vec::new());
    }
    let frame_ms = (1_000.0 / fps).round() as u64;
    let mut segments: Vec<TranscriptSegment> = Vec::new();

    for (top, bottom) in bands {
        let mut previous: Option<Vec<u8>> = None;
        let mut pending: Option<(String, f32, u64, u64)> = None;
        for (index, frame) in frames.iter().enumerate() {
            let band = crop(frame, top, bottom);
            let signature = signature(&band);
            if previous.as_ref().is_some_and(|value| *value == signature) {
                if let Some(value) = pending.as_mut() {
                    value.3 = (index as u64 + 1) * frame_ms;
                }
                continue;
            }
            previous = Some(signature);
            let Some(OcrLine { text, confidence }) =
                engine.recognize(&band.data, band.width, band.height)?
            else {
                pending = None;
                continue;
            };
            let start_ms = index as u64 * frame_ms;
            let end_ms = start_ms + frame_ms;
            match pending.take() {
                Some((pending_text, pending_confidence, pending_start, _))
                    if pending_text == text =>
                {
                    pending = Some((pending_text, pending_confidence.max(confidence), pending_start, end_ms));
                }
                Some((pending_text, pending_confidence, pending_start, pending_end)) => {
                    push_segment(
                        &mut segments,
                        pending_text,
                        pending_confidence,
                        pending_start,
                        pending_end,
                    );
                    pending = Some((text, confidence, start_ms, end_ms));
                }
                None => pending = Some((text, confidence, start_ms, end_ms)),
            }
        }
        if let Some((text, confidence, start_ms, end_ms)) = pending {
            push_segment(&mut segments, text, confidence, start_ms, end_ms);
        }
    }

    segments.sort_by_key(|segment| segment.start_ms);
    for (index, segment) in segments.iter_mut().enumerate() {
        segment.index = index as u32;
    }
    Ok(segments)
}

fn push_segment(
    segments: &mut Vec<TranscriptSegment>,
    text: String,
    confidence: f32,
    start_ms: u64,
    end_ms: u64,
) {
    segments.push(TranscriptSegment {
        index: segments.len() as u32,
        start_ms,
        end_ms: end_ms.max(start_ms),
        text,
        confidence: Some(confidence),
        origin: TranscriptOrigin::HardSubtitleOcr,
        ..TranscriptSegment::default()
    });
}

fn crop(frame: &GrayFrame, top: usize, bottom: usize) -> GrayFrame {
    let bottom = bottom.min(frame.height);
    let mut data = Vec::with_capacity(frame.width * bottom.saturating_sub(top));
    for y in top..bottom {
        data.extend_from_slice(frame.row(y));
    }
    GrayFrame {
        width: frame.width,
        height: bottom.saturating_sub(top),
        data,
    }
}

/// 稀疏采样用于变化门控，避免逐像素比较。
fn signature(frame: &GrayFrame) -> Vec<u8> {
    let mut values = Vec::new();
    for y in 0..frame.height {
        let row = frame.row(y);
        for x in (0..frame.width).step_by(8) {
            values.push(row[x]);
        }
    }
    values
}

#[cfg(test)]
mod tests {
    use super::{detect_bands, GrayFrame};

    fn frame_with_band(text_rows: std::ops::Range<usize>) -> GrayFrame {
        let width = 160;
        let height = 120;
        let mut data = vec![0_u8; width * height];
        for y in text_rows {
            for x in 20..140 {
                data[y * width + x] = 255;
            }
        }
        GrayFrame {
            width,
            height,
            data,
        }
    }

    #[test]
    fn bands_are_detected_in_the_bottom_region_only() {
        let frames = vec![frame_with_band(85..100), frame_with_band(85..100)];

        let bands = detect_bands(&frames);

        assert_eq!(bands.len(), 1);
        assert!(bands[0].0 <= 85 && bands[0].1 >= 100);
    }

    #[test]
    fn blank_frames_produce_no_bands_and_no_segments() {
        let blank = GrayFrame {
            width: 160,
            height: 120,
            data: vec![0_u8; 160 * 120],
        };

        assert!(detect_bands(std::slice::from_ref(&blank)).is_empty());
    }

    #[test]
    fn signatures_change_when_the_band_text_changes() {
        let mut changed = frame_with_band(85..100);
        for x in 20..140 {
            changed.data[87 * 160 + x] = 0;
        }

        assert_ne!(
            super::signature(&frame_with_band(85..100)),
            super::signature(&changed)
        );
    }
}
