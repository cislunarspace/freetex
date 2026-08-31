//! 截图选区窗口管理。
//!
//! Snip selection window management.
//!
//! 选区窗是 `snip.html` 承载的全屏无边框透明窗口；Rust 负责把它摆到
//! 目标显示器上、在确认后换算物理坐标并提交识别任务。MVP 只覆盖
//! 主显示器（多显示器在 roadmap）。
//! The snip window is a fullscreen undecorated transparent window hosting
//! `snip.html`; Rust positions it on the target monitor, converts confirmed
//! coordinates to physical pixels, and submits the job. MVP covers the primary
//! monitor only (multi-monitor is on the roadmap).

use crate::pipeline::Job;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize};

/// 选区矩形（窗口内逻辑像素）。
/// Selection rectangle (logical pixels within the snip window).
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnipRect {
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
}

pub struct SnipManager {
    app: AppHandle,
}

impl SnipManager {
    pub fn new(app: AppHandle) -> Self {
        Self { app }
    }

    /// 开始截图：藏主窗（可选）、摆选区窗到主显示器。
    /// Starts a snip: optionally hides the main window, moves the snip window
    /// onto the primary monitor.
    pub fn start(&self) -> Result<(), String> {
        let config = self
            .app
            .try_state::<crate::config_store::ConfigStore>()
            .map(|s| s.get());
        let hide_main = config
            .as_ref()
            .map(|c| c.snip.hide_main_during_snip)
            .unwrap_or(true);

        let snip = self
            .app
            .get_webview_window("snip")
            .ok_or_else(|| "找不到 snip 窗口".to_string())?;
        let monitor = snip
            .primary_monitor()
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "找不到主显示器".to_string())?;

        if hide_main {
            if let Some(main) = self.app.get_webview_window("main") {
                let _ = main.hide();
            }
        }

        let _ = snip.set_position(PhysicalPosition::new(
            monitor.position().x,
            monitor.position().y,
        ));
        let _ = snip.set_size(PhysicalSize::new(
            monitor.size().width,
            monitor.size().height,
        ));
        let _ = snip.show();
        let _ = snip.set_focus();
        let _ = self.app.emit("snip-shown", serde_json::json!({}));
        Ok(())
    }

    /// 取消截图：藏选区窗、恢复主窗。
    /// Cancels: hides the snip window and restores the main window.
    pub fn cancel(&self) {
        if let Some(snip) = self.app.get_webview_window("snip") {
            let _ = snip.hide();
        }
        if let Some(main) = self.app.get_webview_window("main") {
            let _ = main.show();
            let _ = main.set_focus();
        }
    }

    /// 确认选区：逻辑像素 → 全局物理像素 → 提交识别任务。
    /// Confirms the selection: logical → global physical pixels → submit the job.
    pub fn confirm(&self, rect: SnipRect) -> Result<(), String> {
        let snip = self
            .app
            .get_webview_window("snip")
            .ok_or_else(|| "找不到 snip 窗口".to_string())?;
        let monitor = snip
            .primary_monitor()
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "找不到主显示器".to_string())?;
        let scale = snip.scale_factor().map_err(|e| e.to_string())?;
        let _ = snip.hide();

        let origin = monitor.position();
        let global = Job::SnipRect {
            x: origin.x + (rect.x as f64 * scale).round() as i32,
            y: origin.y + (rect.y as f64 * scale).round() as i32,
            w: (rect.w as f64 * scale).round().max(1.0) as u32,
            h: (rect.h as f64 * scale).round().max(1.0) as u32,
        };
        let submitted = self
            .app
            .try_state::<crate::pipeline_controller::PipelineController>()
            .map(|c| c.submit(global))
            .unwrap_or(false);
        if submitted {
            Ok(())
        } else {
            Err("识别流水线未运行".to_string())
        }
    }
}
