//! 流水线 sink seam：框架无关的事件出口。
//!
//! The pipeline sink seam: a framework-agnostic event outlet.

/// 流水线状态（镜像 altgo 的五态，去掉录音专属两态）。
/// Pipeline status (mirrors altgo's five states minus the recording-only two).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PipelineStatus {
    #[default]
    Idle,
    Recognizing,
    Done,
    Stopped,
}

impl PipelineStatus {
    /// IPC 事件里的字符串形态。
    /// String form used in IPC events.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Recognizing => "recognizing",
            Self::Done => "done",
            Self::Stopped => "stopped",
        }
    }
}

/// 结果来源。
/// Result source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceKind {
    Snip,
    Image,
}

/// 一次成功的识别结果。
/// One successful recognition outcome.
#[derive(Debug, Clone)]
pub struct RecognitionOutcome {
    pub latex: String,
    pub elapse_ms: u64,
    pub source: SourceKind,
}

/// sink seam：生产实现见 `tauri_sink.rs`，测试用 fake 注入。
/// The sink seam: production impl in `tauri_sink.rs`; fakes in tests.
pub trait PipelineSink: Send {
    fn status_changed(&self, status: PipelineStatus);
    fn recognition_succeeded(&self, outcome: &RecognitionOutcome, copy_failed: bool);
    fn failed(&self, message: &str);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_strings_are_stable_ipc_contract() {
        assert_eq!(PipelineStatus::Idle.as_str(), "idle");
        assert_eq!(PipelineStatus::Recognizing.as_str(), "recognizing");
        assert_eq!(PipelineStatus::Done.as_str(), "done");
        assert_eq!(PipelineStatus::Stopped.as_str(), "stopped");
    }
}
