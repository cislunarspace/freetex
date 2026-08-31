//! 图像预处理：与 RapidLaTeXOCR 的 `PreProcess` 逐步对齐。
//!
//! Image preprocessing, step-by-step aligned with RapidLaTeXOCR's `PreProcess`.
//!
//! 流程：`pad`（LA 通道选择 → 反色 → 裁剪文字外接框 → 32 对齐白底填充）→
//! `minmax_size`（超限缩小 + 最小尺寸白底补齐）→ `normalize`。
//! Pipeline: `pad` → `minmax_size` → `normalize`. All pure, all unit-tested.

use crate::error::EngineError;
use image::imageops::FilterType;
use image::{DynamicImage, GrayImage, RgbaImage};

/// 模型输入的归一化参数（来自 RapidLaTeXOCR utils.py）。
/// Normalization constants (from RapidLaTeXOCR utils.py).
pub const MEAN: f32 = 0.7931;
pub const STD: f32 = 0.1738;
/// pad 对齐粒度。
/// Padding divisibility.
const DIVABLE: u32 = 32;
/// 反色判定阈值。
/// Inversion threshold.
const THRESHOLD: f32 = 128.0;

pub const MAX_WIDTH: u32 = 672;
pub const MAX_HEIGHT: u32 = 192;
pub const MIN_WIDTH: u32 = 32;
pub const MIN_HEIGHT: u32 = 32;

/// 任意输入图 → RGBA（透明背景保留给 `pad` 判断）。
/// Any input image → RGBA (transparency kept for `pad` to inspect).
pub fn load_rgba(bytes: &[u8]) -> Result<RgbaImage, EngineError> {
    let img = image::load_from_memory(bytes)
        .map_err(|e| EngineError::Inference(format!("图片解码失败：{e}")))?;
    Ok(img.to_rgba8())
}

/// 亮度的 ITU-R 601-2 加权（对应 PIL `convert("L")` / cv2 RGB2GRAY）。
/// Luma via ITU-R 601-2 weights (matches PIL `convert("L")` / cv2 RGB2GRAY).
pub fn luma(px: image::Rgba<u8>) -> f32 {
    0.299 * px[0] as f32 + 0.587 * px[1] as f32 + 0.114 * px[2] as f32
}

/// RapidLaTeXOCR `pad`：选亮度或反转 alpha 通道，裁出文字外接框，白底 pad 到 32 的倍数。
/// RapidLaTeXOCR `pad`: pick luma or inverted alpha, crop to the text bounding box,
/// pad white up to a multiple of 32.
pub fn pad(img: &RgbaImage) -> GrayImage {
    let (w, h) = img.dimensions();

    // alpha 方差为 0（不透明）取亮度；否则取反转 alpha（文字在最上层）
    // zero alpha variance (opaque) → luma; otherwise inverted alpha (text sits on top)
    let mut alpha_sum = 0.0f64;
    let mut alpha_sq_sum = 0.0f64;
    for px in img.pixels() {
        let a = px[3] as f64;
        alpha_sum += a;
        alpha_sq_sum += a * a;
    }
    let n = (w * h) as f64;
    let alpha_mean = alpha_sum / n;
    let alpha_var = (alpha_sq_sum / n - alpha_mean * alpha_mean).max(0.0);

    let mut data: Vec<f32> = Vec::with_capacity((w * h) as usize);
    if alpha_var < 1e-9 {
        for px in img.pixels() {
            data.push(luma(*px));
        }
    } else {
        for px in img.pixels() {
            data.push(255.0 - px[3] as f32);
        }
    }

    // min-max 归一化到 0..255；均匀图像（span=0）视为空白，直接回退 32x32 白底
    // min-max normalize to 0..255; a uniform image (span=0) is blank → 32x32 white
    let min = data.iter().cloned().fold(f32::INFINITY, f32::min);
    let max = data.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let span = max - min;
    if span < 1e-6 {
        return GrayImage::from_pixel(32, 32, image::Luma([255]));
    }
    for v in &mut data {
        *v = (*v - min) / span * 255.0;
    }

    let mean = data.iter().sum::<f32>() / n as f32;
    let text_bright = mean <= THRESHOLD;

    // 外接框必须在「反转前」的数据上计算（与 Python 一致：mask 先算，data 后反）
    // The bounding box is computed on the PRE-inversion data (Python: mask first,
    // then `data = 255 - data`).
    let is_text = |v: f32| -> bool {
        if text_bright {
            v > THRESHOLD
        } else {
            v < THRESHOLD
        }
    };
    let mut min_x = w;
    let mut min_y = h;
    let mut max_x = 0u32;
    let mut max_y = 0u32;
    let mut found = false;
    for y in 0..h {
        for x in 0..w {
            if is_text(data[(y * w + x) as usize]) {
                found = true;
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
            }
        }
    }
    if text_bright {
        for v in &mut data {
            *v = 255.0 - *v;
        }
    }

    let (rect, rw, rh) = if found {
        let rw = max_x - min_x + 1;
        let rh = max_y - min_y + 1;
        let mut rect = GrayImage::new(rw, rh);
        for y in 0..rh {
            for x in 0..rw {
                let v = data[((min_y + y) * w + min_x + x) as usize];
                rect.put_pixel(x, y, image::Luma([v.round().clamp(0.0, 255.0) as u8]));
            }
        }
        (rect, rw, rh)
    } else {
        // 无文字：退化为 32x32 白底
        // No text found: fall back to 32x32 white
        (GrayImage::from_pixel(32, 32, image::Luma([255])), 32, 32)
    };

    let out_w = DIVABLE * rw.div_ceil(DIVABLE).max(1);
    let out_h = DIVABLE * rh.div_ceil(DIVABLE).max(1);
    let mut padded = GrayImage::from_pixel(out_w, out_h, image::Luma([255]));
    image::imageops::overlay(&mut padded, &rect, 0, 0);
    padded
}

/// RapidLaTeXOCR `minmax_size`：超出 max_dims 等比缩小（双线性），不足 min_dims 白底补齐。
/// RapidLaTeXOCR `minmax_size`: bilinear downscale past max_dims, white-pad up to min_dims.
pub fn minmax_size(img: &GrayImage) -> GrayImage {
    let (w, h) = img.dimensions();
    let mut img = img.clone();

    let ratio_w = w as f64 / MAX_WIDTH as f64;
    let ratio_h = h as f64 / MAX_HEIGHT as f64;
    let max_ratio = ratio_w.max(ratio_h);
    if max_ratio > 1.0 {
        // Python: size = size // max(ratios)，向下取整且至少 1
        // Python: size = size // max(ratios), floored, at least 1
        let nw = ((w as f64 / max_ratio).floor() as u32).max(1);
        let nh = ((h as f64 / max_ratio).floor() as u32).max(1);
        let scaled = image::imageops::resize(&img, nw, nh, FilterType::Triangle);
        img = DynamicImage::from(scaled).to_luma8();
    }

    let (w, h) = img.dimensions();
    let out_w = w.max(MIN_WIDTH);
    let out_h = h.max(MIN_HEIGHT);
    if out_w != w || out_h != h {
        let mut padded = GrayImage::from_pixel(out_w, out_h, image::Luma([255]));
        image::imageops::overlay(&mut padded, &img, 0, 0);
        return padded;
    }
    img
}

/// 归一化单个灰度值：(v - MEAN*255) / (STD*255)。
/// Normalizes one gray value: (v - MEAN*255) / (STD*255).
pub fn normalize_value(v: f32) -> f32 {
    (v - MEAN * 255.0) / (STD * 255.0)
}

/// 把灰度图转成模型输入张量 [1, 1, H, W]（f32，已归一化）。
/// Converts a gray image to the model input tensor [1, 1, H, W] (f32, normalized).
pub fn to_tensor(img: &GrayImage) -> Result<ndarray::Array4<f32>, EngineError> {
    let (w, h) = img.dimensions();
    let mut data = Vec::with_capacity((w * h) as usize);
    for px in img.pixels() {
        data.push(normalize_value(px[0] as f32));
    }
    ndarray::Array4::from_shape_vec((1, 1, h as usize, w as usize), data)
        .map_err(|e| EngineError::Inference(format!("张量构造失败：{e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn white_rgba(w: u32, h: u32) -> RgbaImage {
        RgbaImage::from_pixel(w, h, image::Rgba([255, 255, 255, 255]))
    }

    #[test]
    fn pad_crops_and_aligns_to_32() {
        // 白底 200x100，中间画一个 10x10 黑块
        // White 200x100 with a 10x10 black square in the middle
        let mut img = white_rgba(200, 100);
        for y in 45..55 {
            for x in 95..105 {
                img.put_pixel(x, y, image::Rgba([0, 0, 0, 255]));
            }
        }
        let padded = pad(&img);
        // 外接框 10x10 → pad 到 32x32
        // 10x10 box → padded to 32x32
        assert_eq!(padded.dimensions(), (32, 32));
        // 文字裁剪后贴左上：左上是文字，右下是白底
        // after crop the text sits top-left: dark at origin, white at far corner
        assert!(padded.get_pixel(2, 2)[0] < 100, "左上应为文字");
        assert_eq!(padded.get_pixel(31, 31)[0], 255, "右下应为白底");
    }

    #[test]
    fn pad_inverts_dark_background() {
        // 黑底白字 → pad 内部应反转为白底黑字
        // black background, white text → inverted to white background, dark text
        let mut img = RgbaImage::from_pixel(64, 64, image::Rgba([10, 10, 10, 255]));
        for y in 20..40 {
            for x in 20..40 {
                img.put_pixel(x, y, image::Rgba([240, 240, 240, 255]));
            }
        }
        let padded = pad(&img);
        assert_eq!(padded.dimensions(), (32, 32));
        assert!(padded.get_pixel(0, 0)[0] < 100, "左上是文字（反转后为暗）");
        assert!(padded.get_pixel(31, 31)[0] > 200, "右下应为白底");
    }

    #[test]
    fn pad_handles_transparent_background() {
        // 透明底、不透明黑字：alpha 有方差，走 255 - alpha 通道
        // transparent background with opaque black text: alpha varies → 255 - alpha path
        let mut img = RgbaImage::from_pixel(64, 64, image::Rgba([0, 0, 0, 0]));
        for y in 30..34 {
            for x in 10..50 {
                img.put_pixel(x, y, image::Rgba([0, 0, 0, 255]));
            }
        }
        let padded = pad(&img);
        assert_eq!(padded.dimensions(), (64, 32));
        assert!(padded.get_pixel(0, 0)[0] < 100, "左上是文字");
        assert_eq!(padded.get_pixel(63, 31)[0], 255, "右下应为白底");
    }

    #[test]
    fn pad_blank_image_falls_back_to_white_32() {
        let img = white_rgba(50, 50);
        let padded = pad(&img);
        assert_eq!(padded.dimensions(), (32, 32));
        assert_eq!(padded.get_pixel(0, 0)[0], 255);
    }

    #[test]
    fn minmax_size_scales_down_proportionally() {
        // 1344x384 超过 672x192 两倍 → 缩到 672x192
        // 1344x384 is 2x over → scaled to 672x192
        let img = GrayImage::from_pixel(1344, 384, image::Luma([255]));
        let out = minmax_size(&img);
        assert_eq!(out.dimensions(), (672, 192));
    }

    #[test]
    fn minmax_size_pads_small_images() {
        let img = GrayImage::from_pixel(10, 10, image::Luma([0]));
        let out = minmax_size(&img);
        assert_eq!(out.dimensions(), (32, 32));
        assert_eq!(out.get_pixel(0, 0)[0], 0, "原内容应保留在左上角");
    }

    #[test]
    fn normalize_matches_python_constants() {
        let v = normalize_value(255.0);
        let expected = (255.0 - 0.7931 * 255.0) / (0.1738 * 255.0);
        assert!((v - expected).abs() < 1e-6);
        // 255 → (1-0.7931)/0.1738 ≈ 1.19；0 → -0.7931/0.1738 ≈ -4.56
        // 255 → ≈1.19; 0 → ≈-4.56
        assert!((v - 1.1905).abs() < 1e-3);
        assert!((normalize_value(0.0) + 4.5633).abs() < 1e-3);
    }
}
