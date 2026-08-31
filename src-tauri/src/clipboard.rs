//! 剪贴板输出。
//!
//! Clipboard output.
//!
//! 桌面统一走 arboard（Windows 原生、Linux X11 原生），相比 altgo 的
//! Linux 子进程方案简化为单一实现；Android 上 arboard 需 JNI 宿主环境，
//! 分叉为官方剪贴板插件实现。`copy_html` 承载 MathML 复制：Word 只认
//! `text/html` 剪贴板格式里的公式（Word 粘贴是桌面场景，Android 降级纯文本）。
//! Desktop uses arboard everywhere (native on Windows and X11 Linux) — a single
//! implementation instead of altgo's Linux subprocess approach; on Android,
//! arboard needs a JNI host, so it forks to the official clipboard plugin.
//! `copy_html` carries MathML copies: Word only accepts formulas from the
//! `text/html` clipboard format (a desktop-only scenario; Android degrades to plain text).

use crate::error::ClipboardError;

/// 剪贴板 trait seam，便于测试注入。
/// Clipboard trait seam for test injection.
pub trait Clipboard: Send + Sync {
    fn copy_text(&self, text: &str) -> Result<(), ClipboardError>;
    fn copy_html(&self, html: &str, plain: &str) -> Result<(), ClipboardError>;
}

#[cfg(not(target_os = "android"))]
pub type PlatformClipboard = ArboardClipboard;

#[cfg(not(target_os = "android"))]
pub struct ArboardClipboard;

#[cfg(not(target_os = "android"))]
impl ArboardClipboard {
    /// 构造签名与 Android 实现对齐（arboard 不需要 AppHandle）。
    /// Constructor aligned with the Android impl (arboard needs no AppHandle).
    pub fn new(_app: tauri::AppHandle) -> Self {
        Self
    }
}

#[cfg(not(target_os = "android"))]
impl Clipboard for ArboardClipboard {
    fn copy_text(&self, text: &str) -> Result<(), ClipboardError> {
        let mut cb =
            arboard::Clipboard::new().map_err(|e| ClipboardError::Unavailable(e.to_string()))?;
        cb.set_text(text.to_string())
            .map_err(|e| ClipboardError::Unavailable(e.to_string()))
    }

    fn copy_html(&self, html: &str, plain: &str) -> Result<(), ClipboardError> {
        let mut cb =
            arboard::Clipboard::new().map_err(|e| ClipboardError::Unavailable(e.to_string()))?;
        cb.set_html(html.to_string(), Some(plain.to_string()))
            .map_err(|e| ClipboardError::Unavailable(e.to_string()))
    }
}

/// Android：官方剪贴板插件（JNI 桥接）。
/// Android: the official clipboard plugin (JNI bridged).
#[cfg(target_os = "android")]
pub type PlatformClipboard = TauriClipboard;

#[cfg(target_os = "android")]
pub struct TauriClipboard {
    app: tauri::AppHandle,
}

#[cfg(target_os = "android")]
impl TauriClipboard {
    pub fn new(app: tauri::AppHandle) -> Self {
        Self { app }
    }
}

#[cfg(target_os = "android")]
impl Clipboard for TauriClipboard {
    fn copy_text(&self, text: &str) -> Result<(), ClipboardError> {
        use tauri_plugin_clipboard_manager::ClipboardExt;
        self.app
            .clipboard()
            .write_text(text.to_string())
            .map_err(|e| ClipboardError::Unavailable(e.to_string()))
    }

    /// Android WebView 无 `text/html` 剪贴板需求，降级为纯文本。
    /// Degrades to plain text: the Android WebView has no `text/html` clipboard need.
    fn copy_html(&self, _html: &str, plain: &str) -> Result<(), ClipboardError> {
        self.copy_text(plain)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct ThrowingClipboard;

    impl Clipboard for ThrowingClipboard {
        fn copy_text(&self, _text: &str) -> Result<(), ClipboardError> {
            Err(ClipboardError::Unavailable("test".to_string()))
        }
        fn copy_html(&self, _html: &str, _plain: &str) -> Result<(), ClipboardError> {
            Err(ClipboardError::Unavailable("test".to_string()))
        }
    }

    #[test]
    fn clipboard_trait_is_object_safe() {
        // 保证 trait seam 可被 Box<dyn Clipboard> 持有
        // ensures the trait seam can be held as Box<dyn Clipboard>
        let cb: Box<dyn Clipboard> = Box::new(ThrowingClipboard);
        assert!(cb.copy_text("x").is_err());
        assert!(cb.copy_html("<i>x</i>", "x").is_err());
    }
}
