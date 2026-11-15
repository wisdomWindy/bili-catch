use std::path::Path;

use rten::Model;
use rten_tensor::prelude::*;
use rten_tensor::{NdTensor, Tensor};

use crate::models::{AppError, AppErrorCode};

/// PP-OCRv4 识别模型的固定输入高度。
const REC_HEIGHT: usize = 48;
const REC_MAX_WIDTH: usize = 1_600;
const REC_MIN_WIDTH: usize = 320;

#[derive(Debug, Clone, PartialEq)]
pub struct OcrLine {
    pub text: String,
    pub confidence: f32,
}

pub struct OcrEngine {
    model: Model,
    dictionary: Vec<String>,
}

impl OcrEngine {
    pub fn load(model_path: &Path, dictionary_path: &Path) -> Result<Self, AppError> {
        if !model_path.is_file() || !dictionary_path.is_file() {
            return Err(ocr_error(
                "OCR_MODEL_UNAVAILABLE",
                "The bundled OCR model is unavailable",
            ));
        }
        let model = Model::load_file(model_path)
            .map_err(|_| ocr_error("OCR_MODEL_LOAD_FAILED", "The OCR model could not be loaded"))?;
        let raw = std::fs::read_to_string(dictionary_path).map_err(|_| {
            ocr_error(
                "OCR_DICTIONARY_UNAVAILABLE",
                "The OCR dictionary could not be read",
            )
        })?;
        let dictionary = raw.lines().map(str::to_owned).collect::<Vec<_>>();
        if dictionary.is_empty() {
            return Err(ocr_error(
                "OCR_DICTIONARY_UNAVAILABLE",
                "The OCR dictionary is empty",
            ));
        }
        Ok(Self { model, dictionary })
    }

    /// 识别一行灰度文本。返回 `None` 表示该区域没有可用文本。
    pub fn recognize(
        &self,
        gray: &[u8],
        width: usize,
        height: usize,
    ) -> Result<Option<OcrLine>, AppError> {
        if width == 0 || height == 0 || gray.len() < width * height {
            return Ok(None);
        }
        let (data, target_width) = preprocess(gray, width, height);
        let input = NdTensor::from_data([1, 3, REC_HEIGHT, target_width], data);
        let output = self
            .model
            .run_one(input.into(), None)
            .map_err(|_| ocr_error("OCR_INFERENCE_FAILED", "The OCR model failed to run"))?;
        let tensor: Tensor<f32> = output
            .try_into()
            .map_err(|_| ocr_error("OCR_OUTPUT_INVALID", "The OCR output is invalid"))?;
        let shape = tensor.shape().to_vec();
        if shape.len() != 3 || shape[1] == 0 {
            return Err(ocr_error("OCR_OUTPUT_INVALID", "The OCR output is invalid"));
        }
        let data = tensor.to_vec();
        Ok(decode(&data, &shape, &self.dictionary))
    }
}

fn preprocess(gray: &[u8], width: usize, height: usize) -> (Vec<f32>, usize) {
    let scale = REC_HEIGHT as f32 / height as f32;
    let scaled_width = ((width as f32 * scale).round() as usize)
        .clamp(1, REC_MAX_WIDTH)
        .max(1);
    let target_width = scaled_width.max(REC_MIN_WIDTH);
    let mut data = vec![0.0_f32; 3 * REC_HEIGHT * target_width];

    for y in 0..REC_HEIGHT {
        let source_y = ((y as f32 + 0.5) / scale).min(height as f32 - 1.0);
        for x in 0..scaled_width {
            let source_x = ((x as f32 + 0.5) / scale).min(width as f32 - 1.0);
            let value = bilinear(gray, width, height, source_x, source_y);
            let normalized = (value as f32 / 255.0 - 0.5) / 0.5;
            for channel in 0..3 {
                data[channel * REC_HEIGHT * target_width + y * target_width + x] = normalized;
            }
        }
    }
    (data, target_width)
}

fn bilinear(gray: &[u8], width: usize, height: usize, x: f32, y: f32) -> f32 {
    let x0 = x.floor().max(0.0) as usize;
    let y0 = y.floor().max(0.0) as usize;
    let x1 = (x0 + 1).min(width - 1);
    let y1 = (y0 + 1).min(height - 1);
    let fx = x - x0 as f32;
    let fy = y - y0 as f32;
    let at = |px: usize, py: usize| gray[py * width + px] as f32;
    let top = at(x0, y0) * (1.0 - fx) + at(x1, y0) * fx;
    let bottom = at(x0, y1) * (1.0 - fx) + at(x1, y1) * fx;
    top * (1.0 - fy) + bottom * fy
}

fn decode(data: &[f32], shape: &[usize], dictionary: &[String]) -> Option<OcrLine> {
    let steps = shape[1];
    let classes = shape[2];
    if classes == 0 || data.len() < steps * classes {
        return None;
    }
    let mut text = String::new();
    let mut confidence = 0.0_f32;
    let mut emitted = 0_u32;
    let mut previous = usize::MAX;

    for step in 0..steps {
        let offset = step * classes;
        let row = &data[offset..offset + classes];
        let (mut best, mut best_index) = (f32::MIN, 0_usize);
        for (index, value) in row.iter().enumerate() {
            if *value > best {
                best = *value;
                best_index = index;
            }
        }
        if best_index == 0 || best_index == previous {
            previous = best_index;
            continue;
        }
        previous = best_index;
        if let Some(character) = dictionary.get(best_index - 1) {
            text.push_str(character);
            confidence += softmax_probability(row, best);
            emitted += 1;
        }
    }
    if text.trim().is_empty() || emitted == 0 {
        return None;
    }
    Some(OcrLine {
        text: text.trim().to_owned(),
        confidence: (confidence / emitted as f32).clamp(0.0, 1.0),
    })
}

fn softmax_probability(row: &[f32], best: f32) -> f32 {
    let sum = row
        .iter()
        .map(|value| ((*value - best) as f64).exp())
        .sum::<f64>();
    if sum <= 0.0 {
        return 0.0;
    }
    (1.0 / sum) as f32
}

fn ocr_error(details: &str, message: &str) -> AppError {
    AppError {
        code: AppErrorCode::E014,
        message: message.into(),
        details: Some(details.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::{preprocess, REC_HEIGHT};

    #[test]
    fn preprocessing_keeps_aspect_ratio_and_pads_to_the_minimum_width() {
        let gray = vec![200_u8; 100 * 50];
        let (data, width) = preprocess(&gray, 100, 50);

        assert_eq!(width, 320);
        assert_eq!(data.len(), 3 * REC_HEIGHT * width);
        assert!((data[0] - 0.568_627).abs() < 0.01);
    }

    #[test]
    fn wide_lines_are_scaled_up_to_the_height_and_capped() {
        let gray = vec![128_u8; 2_000 * 50];
        let (_, width) = preprocess(&gray, 2_000, 50);

        assert_eq!(width, 1_600);
    }
}
