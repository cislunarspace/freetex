//! 快捷键生命周期管理（managed state）。
//!
//! Hotkey lifecycle management (managed state).
//!
//! 按下事件在转发线程里触发 `SnipManager::start`；换键 = 重建监听器。
//! Press events trigger `SnipManager::start` on the forwarder thread; changing
//! the key rebuilds the listener.

use crate::config_store::ConfigStore;
use crate::error::HotkeyError;
use crate::hotkey::{HotkeyEvent, HotkeyListener, PlatformListener};
use crate::snip::SnipManager;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager};

fn report_start_failure(app: &AppHandle, err: &HotkeyError) {
    tracing::error!(error = %err, "快捷键监听启动失败");
    let _ = app.emit(
        "pipeline-error",
        serde_json::json!({ "message": format!("快捷键监听启动失败：{err}") }),
    );
}

#[derive(Default)]
pub struct HotkeyManager {
    handle: Mutex<Option<HotkeyHandle>>,
}

struct HotkeyHandle {
    /// 字段本身从不被读：持有它只为让 Drop（卸钩子 / 杀 evtest）在替换时生效。
    /// Never read directly; holding it keeps the Drop side effects (unhook /
    /// kill evtest) alive until the listener is replaced.
    #[allow(dead_code)]
    listener: Box<dyn HotkeyListener>,
}

impl HotkeyManager {
    /// （重）启动快捷键监听；错误经 `pipeline-error` 事件上报给前端。
    /// (Re)starts the hotkey listener; errors surface to the frontend via
    /// `pipeline-error`.
    pub fn restart(&self, app: &AppHandle, key: &str) {
        let mut guard = self.handle.lock().unwrap();
        *guard = None; // drop 旧监听器，转发线程随通道关闭退出
        let Ok(raw_listener) = PlatformListener::new(key) else {
            let err = HotkeyError::UnsupportedKey(format!("无法解析按键 '{key}'"));
            report_start_failure(app, &err);
            return;
        };
        let mut listener = Box::new(raw_listener) as Box<dyn HotkeyListener>;
        match listener.start() {
            Ok((rx, backend)) => {
                let forward_app = app.clone();
                let forward_key = key.to_string();
                let _ = std::thread::Builder::new()
                    .name("freetex-hotkey-forward".into())
                    .spawn(move || {
                        forward_loop(rx, &forward_app, &forward_key);
                    });
                let _ = app.emit("hotkey-backend", serde_json::json!({ "backend": backend }));
                tracing::info!(key, backend, "快捷键监听已启动");
                *guard = Some(HotkeyHandle { listener });
            }
            Err(err) => report_start_failure(app, &err),
        }
    }

    /// 按当前配置启动（lib.rs setup 里调用）。
    /// Starts with the current config (called from lib.rs setup).
    pub fn start_from_config(&self, app: &AppHandle, store: &ConfigStore) {
        let key = store.get().snip.hotkey;
        self.restart(app, &key);
    }
}

/// 转发循环：按下事件 → 截图；通道关闭（监听器被替换）时退出。
/// Forwarder loop: press → snip; exits when the channel closes (listener replaced).
fn forward_loop(rx: std::sync::mpsc::Receiver<HotkeyEvent>, app: &AppHandle, key: &str) {
    loop {
        match rx.recv() {
            Ok(event) if event.pressed => {
                if let Some(manager) = app.try_state::<SnipManager>() {
                    if let Err(err) = manager.start() {
                        tracing::error!(error = %err, "截图流程启动失败");
                    }
                }
            }
            Ok(_) => {}
            Err(_) => {
                tracing::debug!(key, "快捷键转发线程退出");
                break;
            }
        }
    }
}
