//! 快捷键名 → 平台键码映射。
//!
//! Hotkey name → per-platform key-code mapping.
//!
//! `config::SUPPORTED_HOTKEYS` 是唯一键名清单，本模块给出每个名字在
//! Windows（VK 码）与 Linux（evdev 码）下的数值。
//! `config::SUPPORTED_HOTKEYS` owns the name list; this module maps each name to
//! its Windows (VK) and Linux (evdev) numeric codes.

/// 平台键码对（Windows VK，Linux evdev）。
/// Per-platform codes (Windows VK, Linux evdev).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyCodes {
    pub windows_vk: u16,
    pub linux_evdev: u32,
}

/// 解析键名；未知名返回 None。
/// Resolves a key name; None when unknown.
pub fn key_codes(name: &str) -> Option<KeyCodes> {
    // 与 Linux input-event-codes.h / WinUser.h 对应
    // corresponds to Linux input-event-codes.h / WinUser.h
    let (vk, ev) = match name {
        "F1" => (0x70, 58),
        "F2" => (0x71, 59),
        "F3" => (0x72, 60),
        "F4" => (0x73, 61),
        "F5" => (0x74, 62),
        "F6" => (0x75, 63),
        "F7" => (0x76, 64),
        "F8" => (0x77, 65),
        "F9" => (0x78, 66),
        "F10" => (0x79, 68),
        "F11" => (0x7A, 87),
        "F12" => (0x7B, 88),
        "PrintScreen" => (0x2C, 210),
        "Insert" => (0x2D, 110),
        "Delete" => (0x2E, 111),
        "Home" => (0x24, 102),
        "End" => (0x23, 107),
        "PageUp" => (0x21, 104),
        "PageDown" => (0x22, 109),
        "Alt_R" => (0xA5, 100),
        "Control_R" => (0xA3, 97),
        "Shift_R" => (0xA1, 54),
        _ => return None,
    };
    Some(KeyCodes {
        windows_vk: vk,
        linux_evdev: ev,
    })
}

/// evdev 码 → 键名（解析 evtest 输出用；只含支持清单里的键）。
/// evdev code → key name (for parsing evtest output; supported keys only).
pub fn name_from_evdev(code: u32) -> Option<&'static str> {
    crate::config::SUPPORTED_HOTKEYS
        .iter()
        .copied()
        .find(|n| key_codes(n).map(|c| c.linux_evdev == code).unwrap_or(false))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_all_supported_hotkeys() {
        // 配置清单里的每个键都必须有平台码
        // every key in the config list must have platform codes
        for name in crate::config::SUPPORTED_HOTKEYS {
            assert!(key_codes(name).is_some(), "键 {name} 缺少平台码映射");
        }
    }

    #[test]
    fn default_hotkey_is_f9() {
        let codes = key_codes("F9").unwrap();
        assert_eq!(codes.windows_vk, 0x78);
        assert_eq!(codes.linux_evdev, 66);
    }

    #[test]
    fn rejects_unknown_names() {
        assert!(key_codes("Ctrl+Shift+A").is_none());
        assert!(key_codes("").is_none());
    }

    #[test]
    fn evdev_reverse_lookup_roundtrips() {
        assert_eq!(name_from_evdev(66), Some("F9"));
        assert_eq!(name_from_evdev(0xA5), None);
    }
}
