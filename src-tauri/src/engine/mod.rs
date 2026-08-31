//! 公式识别引擎（引擎 seam）。
//!
//! Formula recognition engine (the engine seam).
//!
//! `Recognizer` 是引擎 trait seam；生产实现 `pix2tex::Pix2Tex` 用 ort 跑
//! LaTeX-OCR 的三个 ONNX 模型。`EngineHandle` 包装懒加载与更换模型。
//! `Recognizer` is the engine trait seam; the production impl `pix2tex::Pix2Tex`
//! runs LaTeX-OCR's three ONNX models via ort. `EngineHandle` adds lazy loading.

pub mod pix2tex;
pub mod postprocess;
pub mod preprocess;
pub mod tokenizer;

use crate::error::EngineError;
use std::path::Path;
use std::time::Duration;

/// 一次识别的结果。
/// Result of one recognition.
#[derive(Debug, Clone)]
pub struct Recognition {
    pub latex: String,
    pub elapse: Duration,
}

/// 识别结果消费回调（用于流式/进度扩展；当前实现完成时回调一次）。
/// Recognition completion callback (progress hook for future streaming).
pub trait Recognizer: Send {
    fn recognize(&self, image: &image::RgbaImage) -> Result<Recognition, EngineError>;
}

/// 从模型目录加载生产引擎。
/// Loads the production engine from a model directory.
pub fn load_engine(model_dir: &Path, num_threads: usize) -> Result<pix2tex::Pix2Tex, EngineError> {
    pix2tex::Pix2Tex::load(model_dir, num_threads)
}
