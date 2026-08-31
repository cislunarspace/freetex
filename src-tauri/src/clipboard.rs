//! 剪贴板输出。
//!
//! Clipboard output.
//!
//! 全平台统一走 arboard（Windows 原生、Linux X11 原生），相比 altgo 的
//! Linux 子进程方案简化为单一实现；trait seam 保留，便于以后按平台分叉。
//! All platforms use arboard (native on Windows, native on X11 Linux) — a single
//! implementation instead of altgo's Linux subprocess approach; the trait seam
//! stays for future per-platform forks.
//!
//! `copy_html` 承载 MathML 复制：Word 只认 `text/html` 剪贴板格式里的公式。
//! `copy_html` carries MathML copies: Word only accepts formulas from the
//! `text/html` clipboard format.

use crate::error::ClipboardError;

/// 剪贴板 trait seam，便于测试注入。
/// Clipboard trait seam for test injection.
pub trait Clipboard: Send + Sync {
    fn copy_text(&self, text: &str) -> Result<(), ClipboardError>;
    fn copy_html(&self, html: &str, plain: &str) -> Result<(), ClipboardError>;
}

/// 平台默认实现。
/// Platform default implementation.
pub type PlatformClipboard = ArboardClipboard;

pub struct ArboardClipboard;

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
