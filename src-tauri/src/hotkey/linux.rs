//! Linux 快捷键监听：通过 `evtest` 读 `/dev/input/event*`。
//!
//! Linux hotkey listener: reads `/dev/input/event*` through `evtest`.
//!
//! 需要 `evtest` 与对输入设备的读权限（`input` 组，与 altgo 相同的要求）。
//! Requires `evtest` plus read access to input devices (`input` group, same
//! requirement as altgo).

use super::{keymap, HotkeyEvent, HotkeyListener};
use crate::error::HotkeyError;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{channel, Receiver, Sender};

/// evtest 监听器。
/// The evtest listener.
pub struct EvdevListener {
    evdev_code: u32,
    device: Option<PathBuf>,
    child: Option<Child>,
}

impl EvdevListener {
    pub fn new(key_name: &str) -> Result<Self, HotkeyError> {
        let codes = keymap::key_codes(key_name)
            .ok_or_else(|| HotkeyError::UnsupportedKey(format!("无法解析按键 '{key_name}'")))?;
        Ok(Self {
            evdev_code: codes.linux_evdev,
            device: None,
            child: None,
        })
    }

    /// 找到第一个带按键能力的输入设备。
    /// Finds the first input device with key capability.
    fn find_keyboard_device() -> Option<PathBuf> {
        // 用 evtest --query 探测 EV_KEY 能力（exit 0 = 支持）
        // probes EV_KEY capability via `evtest --query` (exit 0 = supported)
        for i in 0..32 {
            let dev = PathBuf::from(format!("/dev/input/event{i}"));
            if !dev.exists() {
                continue;
            }
            let ok = Command::new("evtest")
                .arg("--query")
                .arg(&dev)
                .arg("EV_KEY")
                .arg("KEY_F9")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .map(|s| s.success())
                .unwrap_or(false);
            if ok {
                return Some(dev);
            }
        }
        None
    }
}

impl HotkeyListener for EvdevListener {
    fn start(&mut self) -> Result<(Receiver<HotkeyEvent>, &'static str), HotkeyError> {
        if self.child.is_some() {
            return Err(HotkeyError::StartFailed(
                "listener already started".to_string(),
            ));
        }
        let device = match self.device.clone() {
            Some(d) => d,
            None => Self::find_keyboard_device().ok_or_else(|| {
                HotkeyError::StartFailed(
                    "未找到可读的键盘设备；请确认已安装 evtest 且当前用户在 input 组".to_string(),
                )
            })?,
        };

        let mut child = Command::new("evtest")
            .arg(&device)
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| HotkeyError::StartFailed(format!("启动 evtest 失败：{e}")))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| HotkeyError::StartFailed("evtest 无标准输出".to_string()))?;

        let (tx, rx) = channel::<HotkeyEvent>();
        let expected_code = self.evdev_code;
        std::thread::Builder::new()
            .name("freetex-evtest".into())
            // ChildStdout 只实现 Read，这里包 BufReader 再进解析线程
            // ChildStdout implements only Read; wrap in a BufReader for the parser thread
            .spawn(move || {
                parse_evtest_stream(BufReader::new(stdout), expected_code, tx)
            })
            .map_err(|e| HotkeyError::StartFailed(format!("解析线程启动失败：{e}")))?;

        self.child = Some(child);
        tracing::info!(device = %device.display(), "evtest 监听已启动");
        Ok((rx, "evtest"))
    }
}

impl Drop for EvdevListener {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

/// 解析 evtest 事件流：`type 1 (EV_KEY) ... value 1`，只转发配置键的事件。
/// Parses the evtest stream (`type 1 (EV_KEY) ... value 1`), forwarding only the
/// configured key's events.
fn parse_evtest_stream<R: BufRead>(reader: R, expected_code: u32, tx: Sender<HotkeyEvent>) {
    for line in reader.lines() {
        let Ok(line) = line else { break };
        if !line.contains("EV_KEY") {
            continue;
        }
        // 值：1 = 按下，0 = 松开，2 = 重复
        // values: 1 = down, 0 = up, 2 = repeat
        let pressed = match line.rsplit("value ").next() {
            Some("1") => true,
            Some("0") => false,
            _ => continue,
        };
        // 只转发配置键的事件，其余键静默忽略
        // forward only the configured key; everything else is ignored
        if extract_code(&line) != Some(expected_code) {
            continue;
        }
        if tx.send(HotkeyEvent { pressed }).is_err() {
            break;
        }
    }
}

/// 从 evtest 行提取 `code N (...)` 的 N。
/// Extracts N from `code N (...)` in an evtest line.
fn extract_code(line: &str) -> Option<u32> {
    let idx = line.find("code ")?;
    let rest = line[idx + 5..].trim_start();
    let num: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    num.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_code_from_evtest_line() {
        let line = "Event: time 1735600000.123456, type 1 (EV_KEY), code 66 (KEY_F9), value 1";
        assert_eq!(extract_code(line), Some(66));
    }

    #[test]
    fn parse_stream_forwards_only_configured_key() {
        let (tx, rx) = channel();
        let input = "Event: time 1.0, type 1 (EV_KEY), code 66 (KEY_F9), value 1\n\
                     Event: time 1.1, type 1 (EV_KEY), code 30 (KEY_A), value 1\n\
                     Event: time 1.2, type 1 (EV_KEY), code 66 (KEY_F9), value 2\n\
                     Event: time 1.3, type 1 (EV_KEY), code 66 (KEY_F9), value 0\n";
        // 配置键为 F9（evdev 66）：只有 F9 的按下/松开被转发
        // configured key is F9 (evdev 66): only F9 down/up get forwarded
        parse_evtest_stream(input.as_bytes(), 66, tx);
        assert_eq!(rx.recv().unwrap(), HotkeyEvent { pressed: true });
        assert_eq!(rx.recv().unwrap(), HotkeyEvent { pressed: false });
        assert!(rx.recv().is_err(), "KEY_A 与重复事件不应转发");
    }

    #[test]
    fn parse_stream_ignores_other_supported_keys() {
        let (tx, rx) = channel();
        let input = "Event: time 1.0, type 1 (EV_KEY), code 100 (KEY_RIGHTALT), value 1\n";
        // 配置键为 F9，右 Alt 按下不应触发
        // configured key is F9; right-Alt presses must not trigger
        parse_evtest_stream(input.as_bytes(), 66, tx);
        assert!(rx.recv().is_err(), "非配置键不应转发");
    }

    #[test]
    fn new_rejects_unknown_key() {
        assert!(EvdevListener::new("nope").is_err());
    }
}
