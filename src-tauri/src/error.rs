//! 错误模型：两层（致命 / 可恢复），各模块持有自己的 thiserror 枚举。
//!
//! Error model: two tiers (fatal / recoverable), with each module owning its own
//! thiserror enum.
//!
//! - `FatalError`：流水线构建期问题，不启动识别线程。
//! - `RecoverableError`：运行期单次失败，降级继续运行。

use thiserror::Error;

/// 致命错误：识别线程无法启动。
/// Fatal errors: the worker thread cannot start.
#[derive(Debug, Error)]
pub enum FatalError {
    #[error("引擎初始化失败：{0}")]
    EngineInit(String),
}

/// 可恢复错误：单次识别失败，管道继续运行。
/// Recoverable errors: a single recognition failure; the pipeline keeps running.
#[derive(Debug, Error)]
pub enum RecoverableError {
    #[error("识别失败：{0}")]
    Recognition(String),
    #[error("截图失败：{0}")]
    Capture(String),
    #[error("剪贴板写入失败：{0}")]
    Clipboard(String),
}

/// 流水线顶层错误。
/// Top-level pipeline error.
#[derive(Debug, Error)]
pub enum PipelineError {
    #[error(transparent)]
    Fatal(#[from] FatalError),
    #[error(transparent)]
    Recoverable(#[from] RecoverableError),
}

#[derive(Debug, Error)]
pub enum EngineError {
    #[error("模型文件缺失：{0}")]
    ModelMissing(String),
    #[error("模型加载失败：{0}")]
    ModelLoad(String),
    #[error("推理失败：{0}")]
    Inference(String),
}

#[derive(Debug, Error)]
pub enum ModelError {
    #[error("不支持的模型：{0}")]
    UnknownModel(String),
    #[error("下载失败：{0}")]
    Download(String),
    #[error("文件校验失败：{0}")]
    Verification(String),
    #[error("IO 错误：{0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("配置校验失败：{0}")]
    Validation(String),
    #[error("配置读写失败：{0}")]
    Io(#[from] std::io::Error),
    #[error("配置解析失败：{0}")]
    Parse(String),
}

#[derive(Debug, Error)]
pub enum HistoryError {
    #[error("历史读写失败：{0}")]
    Io(#[from] std::io::Error),
    #[error("历史解析失败：{0}")]
    Parse(String),
}

#[derive(Debug, Error)]
pub enum ClipboardError {
    #[error("剪贴板不可用：{0}")]
    Unavailable(String),
}

#[derive(Debug, Error)]
pub enum CaptureError {
    #[error("屏幕捕获失败：{0}")]
    Capture(String),
    #[error("图像处理失败：{0}")]
    Image(#[from] image::ImageError),
}

#[derive(Debug, Error)]
pub enum HotkeyError {
    #[error("热键启动失败：{0}")]
    StartFailed(String),
    #[error("不支持的热键：{0}")]
    UnsupportedKey(String),
}
