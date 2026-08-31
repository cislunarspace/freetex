//! `PipelineSink` 的 Tauri 适配器：管道事件 → 前端事件。
//!
//! Tauri adapter for `PipelineSink`: pipeline events → frontend events.
//!
//! 事件契约（前端依赖的字段名，camelCase）：
//! Event contract (camelCase field names the frontend relies on):
//! - `pipeline-status` `{ status }`
//! - `recognition-result` `{ latex, elapseMs, source, copyFailed }`
//! - `pipeline-error` `{ message }`
//!
//! 识别成功/失败后都会重新显示主窗口（截图流程把它藏起来了）。
//! After success or failure the main window is shown again (the snip flow hid it).

use crate::pipeline::sink::{PipelineSink, PipelineStatus, RecognitionOutcome};
use std::sync::{Arc, RwLock};
use tauri::{AppHandle, Emitter, Manager};

pub struct TauriSink {
    app: AppHandle,
    /// 共享状态：除发事件外，同步落到 PipelineController 供 updater 等读取。
    /// Shared status: besides emitting, mirrored into PipelineController for
    /// outsiders like the updater.
    status: Arc<RwLock<PipelineStatus>>,
}

impl TauriSink {
    pub fn new(app: AppHandle, status: Arc<RwLock<PipelineStatus>>) -> Self {
        *status.write().unwrap() = PipelineStatus::Idle;
        Self { app, status }
    }

    fn show_main(&self) {
        if let Some(main) = self.app.get_webview_window("main") {
            let _ = main.show();
            let _ = main.unminimize();
            let _ = main.set_focus();
        }
    }
}

impl PipelineSink for TauriSink {
    fn status_changed(&self, status: PipelineStatus) {
        *self.status.write().unwrap() = status;
        let _ = self.app.emit(
            "pipeline-status",
            serde_json::json!({ "status": status.as_str() }),
        );
    }

    fn recognition_succeeded(&self, outcome: &RecognitionOutcome, copy_failed: bool) {
        self.show_main();
        let _ = self.app.emit(
            "recognition-result",
            serde_json::json!({
                "latex": outcome.latex,
                "elapseMs": outcome.elapse_ms,
                "source": match outcome.source {
                    crate::pipeline::sink::SourceKind::Snip => "snip",
                    crate::pipeline::sink::SourceKind::Image => "image",
                },
                "copyFailed": copy_failed,
            }),
        );
        let _ = self.app.emit("history-updated", serde_json::json!({}));
    }

    fn failed(&self, message: &str) {
        self.show_main();
        let _ = self
            .app
            .emit("pipeline-error", serde_json::json!({ "message": message }));
    }
}
