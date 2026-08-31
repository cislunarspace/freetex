//! 快捷键监听（触发截图识别）。
//!
//! Hotkey listening (triggers a snip).
//!
//! 移植 altgo 的 `key_listener` 架构：Windows 用 `WH_KEYBOARD_LL` 低级钩子，
//! Linux 用 `evtest` 读 `/dev/input/event*`。freetex 只关心「按下」这一件事，
//! 不需要 altgo 的长按/双击状态机。
//! Ported from altgo's `key_listener` architecture: Windows uses a `WH_KEYBOARD_LL`
//! hook, Linux reads `/dev/input/event*` via `evtest`. freetex only cares about
//! key-down — no long-press/double-click state machine needed.

pub mod keymap;

#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(target_os = "windows")]
pub mod windows;

#[cfg(target_os = "linux")]
pub use linux::EvdevListener;
#[cfg(target_os = "windows")]
pub use windows::WindowsListener;

#[cfg(target_os = "linux")]
pub type PlatformListener = EvdevListener;
#[cfg(target_os = "windows")]
pub type PlatformListener = WindowsListener;

use crate::error::HotkeyError;
use std::sync::mpsc::Receiver;

/// 快捷键事件。
/// Hotkey event.
#[derive(Debug, Clone, PartialEq)]
pub struct HotkeyEvent {
    pub pressed: bool,
}

/// 持续监听快捷键的 trait seam。
/// Hotkey-listener trait seam.
pub trait HotkeyListener: Send {
    /// 开始监听，返回事件通道与后端标识（如 `"windows-hook"`）。
    /// Starts listening; returns the event channel plus a backend label.
    fn start(&mut self) -> Result<(Receiver<HotkeyEvent>, &'static str), HotkeyError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeListener;

    impl HotkeyListener for FakeListener {
        fn start(&mut self) -> Result<(Receiver<HotkeyEvent>, &'static str), HotkeyError> {
            let (tx, rx) = std::sync::mpsc::channel();
            let _ = tx.send(HotkeyEvent { pressed: true });
            Ok((rx, "fake"))
        }
    }

    #[test]
    fn trait_seam_is_boxable_and_delivers_events() {
        let mut listener: Box<dyn HotkeyListener> = Box::new(FakeListener);
        let (rx, backend) = listener.start().unwrap();
        assert_eq!(backend, "fake");
        assert_eq!(rx.recv().unwrap(), HotkeyEvent { pressed: true });
    }
}
