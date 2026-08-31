//! LaTeX-OCR（pix2tex）ONNX 推理实现。
//!
//! LaTeX-OCR (pix2tex) ONNX inference via ort.
//!
//! 三个模型（RapidAI/RapidLaTeXOCR v0.0.0 转换）：
//! Three models (converted by RapidAI/RapidLaTeXOCR v0.0.0):
//! - `image_resizer.onnx`：预测适配宽度档位（宽 = (argmax+1)*32）
//! - `encoder.onnx`：图像 → 上下文张量
//! - `decoder.onnx`：自回归解码（无 KV cache，每步喂全序列取最后一位 logits）
//!
//! 解码为 top-k 采样（k = 10% 词表），配置温度 1e-5 下饱和为贪心。
//! Decoding is top-k sampling (k = 10% of the vocab); at the configured temperature
//! 1e-5 the softmax saturates to greedy.
//!
//! ort 输入按模型元数据的输入名绑定（不写死名字，兼容转换版本差异）。
//! ort inputs are bound by the models' own input names (never hardcoded), tolerating
//! differences between conversion revisions.
//!
//! ort 的 `Session::run` 需要 `&mut self` 且不可克隆，因此三个会话包在 `Mutex` 里；
//! 识别流水线本身单任务串行，锁无竞争。
//! ort's `Session::run` takes `&mut self` and `Session` is not `Clone`, so the three
//! sessions live in `Mutex`es; the pipeline is single-job serial anyway, so no contention.

use super::preprocess::{self};
use super::tokenizer::Tokenizer;
use super::{Recognition, Recognizer};
use crate::error::EngineError;
use image::imageops::FilterType;
use image::{GrayImage, RgbaImage};
use ort::session::builder::GraphOptimizationLevel;
use ort::session::Session;
use ort::value::Tensor;
use rand::RngExt;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Instant;

/// BOS / EOS token id（与 RapidLaTeXOCR config.yaml 一致）。
/// BOS / EOS token ids (same as RapidLaTeXOCR config.yaml).
const BOS_TOKEN: u32 = 1;
const EOS_TOKEN: u32 = 2;
/// 最大解码步数。
/// Maximum decode steps.
const MAX_SEQ_LEN: usize = 512;
/// 采样温度：1e-5 下 softmax 饱和为贪心。
/// Sampling temperature: softmax saturates to greedy at 1e-5.
const TEMPERATURE: f64 = 1e-5;
/// top-k 阈值：保留前 10% logits。
/// Top-k threshold: keep the top 10% logits.
const TOP_K_THRES: f64 = 0.9;

fn load_session(path: PathBuf, num_threads: usize) -> Result<Mutex<Session>, EngineError> {
    let threads = crate::resource::effective_threads(num_threads);
    let session = Session::builder()
        .map_err(|e| EngineError::ModelLoad(e.to_string()))?
        .with_optimization_level(GraphOptimizationLevel::Level3)
        .map_err(|e| EngineError::ModelLoad(e.to_string()))?
        .with_intra_threads(threads)
        .map_err(|e| EngineError::ModelLoad(e.to_string()))?
        .commit_from_file(&path)
        .map_err(|e| EngineError::ModelLoad(format!("{}：{e}", path.display())))?;
    Ok(Mutex::new(session))
}

pub struct Pix2Tex {
    resizer: Mutex<Session>,
    encoder: Mutex<Session>,
    decoder: Mutex<Session>,
    tokenizer: Tokenizer,
}

impl Pix2Tex {
    /// 从模型目录加载三个 ONNX 会话与 tokenizer。
    /// Loads the three ONNX sessions and the tokenizer from a model directory.
    pub fn load(model_dir: &Path, num_threads: usize) -> Result<Self, EngineError> {
        let must = |name: &str| -> Result<PathBuf, EngineError> {
            let p = model_dir.join(name);
            if !p.is_file() {
                return Err(EngineError::ModelMissing(p.display().to_string()));
            }
            Ok(p)
        };

        Ok(Self {
            resizer: load_session(must("image_resizer.onnx")?, num_threads)?,
            encoder: load_session(must("encoder.onnx")?, num_threads)?,
            decoder: load_session(must("decoder.onnx")?, num_threads)?,
            tokenizer: Tokenizer::from_file(must("tokenizer.json")?)
                .map_err(EngineError::ModelLoad)?,
        })
    }

    /// 识别入口：RGBA 图 → LaTeX。
    /// Entry point: RGBA image → LaTeX.
    pub fn recognize_rgba(&self, img: &RgbaImage) -> Result<Recognition, EngineError> {
        let start = Instant::now();
        let latex = self.run(img)?;
        Ok(Recognition {
            latex,
            elapse: start.elapsed(),
        })
    }

    fn run(&self, img: &RgbaImage) -> Result<String, EngineError> {
        let tensor = self.prepare_input(img)?;
        let context = self.encode(tensor)?;
        let ids = self.decode(&context)?;
        let text = self.tokenizer.decode(&ids);
        Ok(super::postprocess::post_process(&text))
    }

    /// 预处理 + resizer 宽度适配循环，返回 encoder 输入张量。
    /// Preprocessing + resizer width-fitting loop; returns the encoder input tensor.
    fn prepare_input(&self, img: &RgbaImage) -> Result<ndarray::Array4<f32>, EngineError> {
        let base = preprocess::minmax_size(&preprocess::pad(img));

        // 与 Python loop_image_resizer 一致：先按原尺寸预测，再按比例迭代收敛
        // Same as Python's loop_image_resizer: predict at original size, then iterate
        let mut r = 1.0f64;
        let (mut w, mut h) = base.dimensions();
        let mut final_tensor: Option<ndarray::Array4<f32>> = None;

        for _ in 0..10 {
            // Python: h = int(h * r)；截断取整，最低保 1 防崩溃
            // Python: h = int(h * r); truncating, floored at 1 to avoid crashes
            h = ((h as f64 * r) as u32).max(1);
            let (tensor, pad_dims) = self.pre_process_round(&base, w, h, r)?;
            let bucket = self.predict_width_bucket(&tensor)?;
            w = bucket * 32;
            let converged = w == pad_dims.0;
            final_tensor = Some(tensor);
            if converged {
                break;
            }
            r = w as f64 / pad_dims.0 as f64;
        }

        final_tensor.ok_or_else(|| EngineError::Inference("resizer 循环未产出张量".to_string()))
    }

    /// 一轮 resizer 循环：缩放到 (w, h) → minmax/pad → 归一化张量，返回张量与 pad 后尺寸。
    /// One resizer round: scale to (w, h) → minmax/pad → normalized tensor; returns
    /// the tensor and the padded dimensions.
    fn pre_process_round(
        &self,
        base: &GrayImage,
        w: u32,
        h: u32,
        r: f64,
    ) -> Result<(ndarray::Array4<f32>, (u32, u32)), EngineError> {
        // Python：r > 1 用 BILINEAR，否则 LANCZOS
        // Python: BILINEAR when r > 1, LANCZOS otherwise
        let filter = if r > 1.0 {
            FilterType::Triangle
        } else {
            FilterType::Lanczos3
        };
        let resized = image::imageops::resize(base, w.max(1), h.max(1), filter);
        let fitted = preprocess::minmax_size(&resized);
        let padded = preprocess::pad(&to_rgba_opaque(&fitted));
        let dims = padded.dimensions();
        let tensor = preprocess::to_tensor(&padded)?;
        Ok((tensor, dims))
    }

    /// resizer：图像张量 → 宽度档位（(argmax+1)*32）。
    /// resizer: image tensor → width bucket ((argmax+1)*32).
    fn predict_width_bucket(&self, tensor: &ndarray::Array4<f32>) -> Result<u32, EngineError> {
        let data = self.run_single(&self.resizer, tensor, "image_resizer")?;
        let argmax = data
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(i, _)| i)
            .ok_or_else(|| EngineError::Inference("resizer 输出为空".to_string()))?;
        Ok((argmax + 1) as u32)
    }

    /// encoder：图像张量 → 上下文（owned ArrayD）。
    /// encoder: image tensor → context (owned ArrayD).
    fn encode(&self, tensor: ndarray::Array4<f32>) -> Result<ndarray::ArrayD<f32>, EngineError> {
        let mut session = self.encoder.lock().unwrap();
        let name = session.inputs()[0].name().to_string();
        let out_name = session.outputs()[0].name().to_string();
        let input = Tensor::from_array(tensor)
            .map_err(|e| EngineError::Inference(format!("encoder 输入构造失败：{e}")))?;
        let outputs = session
            .run(ort::inputs![name.as_str() => input])
            .map_err(|e| EngineError::Inference(format!("encoder 推理失败：{e}")))?;
        let view = outputs[out_name.as_str()]
            .try_extract_array::<f32>()
            .map_err(|e| EngineError::Inference(format!("encoder 输出提取失败：{e}")))?;
        Ok(view.to_owned())
    }

    /// decoder 自回归循环：返回生成的 token id 序列（不含 BOS，含 EOS）。
    /// Autoregressive decoder loop; returns generated ids (BOS stripped, EOS kept).
    fn decode(&self, context: &ndarray::ArrayD<f32>) -> Result<Vec<u32>, EngineError> {
        let context_tensor = Tensor::from_array(context.to_owned())
            .map_err(|e| EngineError::Inference(format!("context 张量构造失败：{e}")))?;
        let (in_ids, in_mask, in_ctx) = {
            let session = self.decoder.lock().unwrap();
            let inputs = session.inputs();
            if inputs.len() < 3 {
                return Err(EngineError::ModelLoad(format!(
                    "decoder 期望 3 个输入，实际 {} 个",
                    inputs.len()
                )));
            }
            (
                inputs[0].name().to_string(),
                inputs[1].name().to_string(),
                inputs[2].name().to_string(),
            )
        };

        let mut out: Vec<u32> = vec![BOS_TOKEN];
        for _ in 0..MAX_SEQ_LEN {
            // 每步喂全部已生成序列（窗口上限 MAX_SEQ_LEN），取最后一位 logits
            // feed the full generated sequence each step (window capped), take last-position logits
            let t = out.len().min(MAX_SEQ_LEN);
            let window: Vec<i64> = out[out.len() - t..].iter().map(|&v| v as i64).collect();
            let mask: Vec<bool> = vec![true; t];

            let ids_tensor = Tensor::from_array((vec![1i64, t as i64], window))
                .map_err(|e| EngineError::Inference(format!("ids 张量构造失败：{e}")))?;
            let mask_tensor = Tensor::from_array((vec![1i64, t as i64], mask))
                .map_err(|e| EngineError::Inference(format!("mask 张量构造失败：{e}")))?;

            let next = {
                let mut session = self.decoder.lock().unwrap();
                let out_name = session.outputs()[0].name().to_string();
                let outputs = session
                    .run(ort::inputs![
                        in_ids.as_str() => ids_tensor,
                        in_mask.as_str() => mask_tensor,
                        in_ctx.as_str() => &context_tensor,
                    ])
                    .map_err(|e| EngineError::Inference(format!("decoder 推理失败：{e}")))?;
                let view = outputs[out_name.as_str()]
                    .try_extract_array::<f32>()
                    .map_err(|e| EngineError::Inference(format!("decoder 输出提取失败：{e}")))?;
                let vocab = *view
                    .shape()
                    .last()
                    .ok_or_else(|| EngineError::Inference("decoder 输出维度为空".to_string()))?;
                let start = (t - 1) * vocab;
                let logits: Vec<f32> = view.iter().skip(start).take(vocab).cloned().collect();
                sample_top_k(&logits)
            };

            out.push(next);
            if next == EOS_TOKEN {
                break;
            }
        }
        Ok(out[1..].to_vec())
    }

    /// 单输入会话执行（resizer / encoder 共用），返回首个输出展平数据。
    /// Runs a single-input session (shared by resizer / encoder); returns the first
    /// output flattened.
    fn run_single(
        &self,
        session: &Mutex<Session>,
        tensor: &ndarray::Array4<f32>,
        label: &str,
    ) -> Result<Vec<f32>, EngineError> {
        let mut guard = session.lock().unwrap();
        let name = guard.inputs()[0].name().to_string();
        let out_name = guard.outputs()[0].name().to_string();
        let input = Tensor::from_array(tensor.clone())
            .map_err(|e| EngineError::Inference(format!("{label} 输入构造失败：{e}")))?;
        let outputs = guard
            .run(ort::inputs![name.as_str() => input])
            .map_err(|e| EngineError::Inference(format!("{label} 推理失败：{e}")))?;
        let view = outputs[out_name.as_str()]
            .try_extract_array::<f32>()
            .map_err(|e| EngineError::Inference(format!("{label} 输出提取失败：{e}")))?;
        Ok(view.iter().cloned().collect())
    }
}

impl Recognizer for Pix2Tex {
    fn recognize(&self, image: &RgbaImage) -> Result<Recognition, EngineError> {
        self.recognize_rgba(image)
    }
}

/// 灰度图 → 不透明 RGBA（pad 需要统一的 alpha 判断入口）。
/// Gray → opaque RGBA (pad expects a uniform alpha-aware entry point).
fn to_rgba_opaque(img: &GrayImage) -> RgbaImage {
    let mut out = RgbaImage::new(img.width(), img.height());
    for (x, y, px) in img.enumerate_pixels() {
        out.put_pixel(x, y, image::Rgba([px[0], px[0], px[0], 255]));
    }
    out
}

/// top-k 过滤 + 温度 softmax + 多项式采样，返回下一个 token id。
/// Top-k filter + temperature softmax + multinomial sampling; returns the next token id.
fn sample_top_k(logits: &[f32]) -> u32 {
    let vocab = logits.len();
    let k = (((1.0 - TOP_K_THRES) * vocab as f64) as usize).clamp(1, vocab.max(1));
    let mut idx: Vec<usize> = (0..vocab).collect();
    idx.sort_unstable_by(|&a, &b| {
        logits[b]
            .partial_cmp(&logits[a])
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    idx.truncate(k);

    let max = idx
        .iter()
        .map(|&i| logits[i] as f64)
        .fold(f64::NEG_INFINITY, f64::max);
    // 数值稳定 softmax；温度极小时非最大项下溢为 0，等价贪心
    // numerically stable softmax; at tiny temperature non-max terms underflow to 0 (greedy)
    let weights: Vec<f64> = idx
        .iter()
        .map(|&i| ((logits[i] as f64 - max) / TEMPERATURE).exp())
        .collect();
    let sum: f64 = weights.iter().sum();
    if sum <= 0.0 {
        return idx[0] as u32;
    }
    let mut r = rand::rng().random::<f64>() * sum;
    for (j, &i) in idx.iter().enumerate() {
        r -= weights[j];
        if r <= 0.0 {
            return i as u32;
        }
    }
    idx[0] as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_top_k_prefers_max_at_tiny_temperature() {
        let mut logits = vec![0.0f32; 1000];
        logits[733] = 42.0;
        for _ in 0..20 {
            assert_eq!(sample_top_k(&logits), 733);
        }
    }

    #[test]
    fn sample_top_k_handles_short_vocab() {
        let logits = vec![0.1f32, 3.0, 0.5];
        assert_eq!(sample_top_k(&logits), 1);
    }

    /// 端到端：加载本地模型识别官方测试图。
    /// End-to-end: load local models and recognize official test images.
    ///
    /// 需要 `.dev/models/` 与 `.dev/test-images/`（见 .gitignore）；无模型时跳过。
    /// Requires `.dev/models/` and `.dev/test-images/`; skipped when models absent.
    #[test]
    #[ignore = "需要本地模型：cargo test --lib e2e -- --ignored --nocapture"]
    fn e2e_recognizes_official_test_images() {
        let model_dir = std::path::Path::new("../.dev/models");
        if !model_dir.join("encoder.onnx").exists() {
            eprintln!("跳过：本地模型不存在");
            return;
        }
        let engine = Pix2Tex::load(model_dir, 0).expect("模型加载失败");
        for name in ["2.png", "6.png", "1.png", "5.png"] {
            let path = std::path::PathBuf::from("../.dev/test-images").join(name);
            let Ok(bytes) = std::fs::read(&path) else {
                eprintln!("跳过：{name} 不存在");
                continue;
            };
            let img = super::super::preprocess::load_rgba(&bytes).expect("图片解码失败");
            let rec = engine
                .recognize_rgba(&img)
                .unwrap_or_else(|e| panic!("{name} 识别失败：{e}"));
            println!("{name} => {} ({} ms)", rec.latex, rec.elapse.as_millis());
            assert!(!rec.latex.trim().is_empty(), "{name} 输出为空");
            assert!(rec.latex.len() >= 2, "{name} 输出疑似过短：{}", rec.latex);
        }
    }
}
