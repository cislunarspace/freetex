//! Windows 快捷键监听（移植自 altgo `key_listener/windows.rs`）。
//!
//! Windows hotkey listener (ported from altgo's `key_listener/windows.rs`).
//!
//! `WH_KEYBOARD_LL` 低级钩子在专属线程上接收全局按键；回调只做 VK 匹配与
//! 通道发送，保证快速返回（超时会被系统摘除钩子）。注入事件被过滤。
//! The `WH_KEYBOARD_LL` hook receives global keys on a dedicated thread; the
//! callback only matches VKs and sends down a channel so it returns fast (a
//! timeout gets the hook stripped). Injected events are filtered.

use super::{keymap, HotkeyEvent, HotkeyListener};
use crate::error::HotkeyError;
use std::sync::mpsc::{channel, Receiver};
use std::thread::JoinHandle;
use windows::Win32::Foundation::{HINSTANCE, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, GetMessageW, PostThreadMessageW, SetWindowsHookExW, UnhookWindowsHookEx,
    KBDLLHOOKSTRUCT, LLKHF_INJECTED, WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP, WM_QUIT, WM_SYSKEYDOWN,
    WM_SYSKEYUP,
};

/// 钩子线程回调：`(vk_code, pressed)`，仅物理按键。
/// Hook-thread callback: `(vk_code, pressed)`, physical keys only.
type HookCallback = Box<dyn Fn(u16, bool) + Send>;

thread_local! {
    static HOOK_CALLBACK: std::cell::RefCell<Option<HookCallback>> =
        const { std::cell::RefCell::new(None) };
}

unsafe extern "system" fn ll_keyboard_proc(
    n_code: i32,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    if n_code >= 0 {
        let msg = w_param.0 as u32;
        let pressed = match msg {
            WM_KEYDOWN | WM_SYSKEYDOWN => true,
            WM_KEYUP | WM_SYSKEYUP => false,
            _ => return CallNextHookEx(None, n_code, w_param, l_param),
        };
        let info = &*(l_param.0 as *const KBDLLHOOKSTRUCT);
        // 忽略注入的合成事件
        // ignore synthetic injected events
        if (info.flags.0 & LLKHF_INJECTED.0) == 0 {
            HOOK_CALLBACK.with(|cb| {
                if let Some(cb) = cb.borrow().as_ref() {
                    cb(info.vkCode as u16, pressed);
                }
            });
        }
    }
    CallNextHookEx(None, n_code, w_param, l_param)
}

/// 运行中的钩子句柄；drop 时停止钩子线程。
/// Running hook handle; dropping it stops the hook thread.
#[derive(Debug)]
struct HookHandle {
    thread_id: u32,
    thread: Option<JoinHandle<()>>,
}

impl HookHandle {
    fn stop(mut self) {
        unsafe {
            let _ = PostThreadMessageW(self.thread_id, WM_QUIT, WPARAM(0), LPARAM(0));
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

impl Drop for HookHandle {
    fn drop(&mut self) {
        unsafe {
            let _ = PostThreadMessageW(self.thread_id, WM_QUIT, WPARAM(0), LPARAM(0));
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// 在新线程上安装钩子并运行消息循环；`on_event` 必须保持轻量。
/// Installs the hook on a new thread and runs its message loop; `on_event` must
/// stay lightweight.
fn spawn_ll_keyboard_hook<F>(on_event: F) -> Result<HookHandle, String>
where
    F: Fn(u16, bool) + Send + 'static,
{
    let (ready_tx, ready_rx) = channel::<Result<u32, String>>();
    let thread = std::thread::Builder::new()
        .name("freetex-keyboard-hook".into())
        .spawn(move || {
            HOOK_CALLBACK.with(|cb| *cb.borrow_mut() = Some(Box::new(on_event)));
            let thread_id = unsafe { GetCurrentThreadId() };

            unsafe {
                let hook = SetWindowsHookExW(
                    WH_KEYBOARD_LL,
                    Some(ll_keyboard_proc),
                    HINSTANCE::default(),
                    0,
                )
                .map_err(|err| format!("SetWindowsHookExW failed: {err}"));
                let hook = match hook {
                    Ok(h) => h,
                    Err(e) => {
                        let _ = ready_tx.send(Err(e));
                        return;
                    }
                };
                if ready_tx.send(Ok(thread_id)).is_err() {
                    let _ = UnhookWindowsHookEx(hook);
                    return;
                }

                let mut msg = Default::default();
                // GetMessageW 返回 0 表示收到 WM_QUIT
                // GetMessageW returning 0 means WM_QUIT received
                while GetMessageW(&mut msg, None, 0, 0).0 > 0 {}
                let _ = UnhookWindowsHookEx(hook);
            }
        })
        .map_err(|e| format!("failed to spawn hook thread: {e}"))?;

    let thread_id = ready_rx
        .recv()
        .map_err(|_| "hook thread exited before ready".to_string())??;

    Ok(HookHandle {
        thread_id,
        thread: Some(thread),
    })
}

/// Windows 快捷键监听器。
/// The Windows hotkey listener.
#[derive(Debug)]
pub struct WindowsListener {
    vk_code: u16,
    key_name: String,
    hook: Option<HookHandle>,
}

impl WindowsListener {
    pub fn new(key_name: &str) -> Result<Self, HotkeyError> {
        let codes = keymap::key_codes(key_name).ok_or_else(|| {
            HotkeyError::UnsupportedKey(format!("无法将按键 '{key_name}' 解析为 Windows VK 码"))
        })?;
        Ok(Self {
            vk_code: codes.windows_vk,
            key_name: key_name.to_string(),
            hook: None,
        })
    }
}

impl HotkeyListener for WindowsListener {
    fn start(&mut self) -> Result<(Receiver<HotkeyEvent>, &'static str), HotkeyError> {
        if self.hook.is_some() {
            return Err(HotkeyError::StartFailed(
                "listener already started".to_string(),
            ));
        }
        let (tx, rx) = channel();
        let vk = self.vk_code;
        let forward = move |event_vk: u16, pressed: bool| {
            if event_vk == vk {
                let _ = tx.send(HotkeyEvent { pressed });
            }
        };
        self.hook = Some(spawn_ll_keyboard_hook(forward).map_err(HotkeyError::StartFailed)?);
        tracing::info!(
            key = %self.key_name,
            vk = format!("0x{:02X}", self.vk_code),
            "Windows keyboard hook installed"
        );
        Ok((rx, "windows-hook"))
    }
}

impl Drop for WindowsListener {
    fn drop(&mut self) {
        if let Some(hook) = self.hook.take() {
            hook.stop();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_resolves_default_hotkey() {
        let listener = WindowsListener::new("F9").unwrap();
        assert_eq!(listener.vk_code, 0x78);
    }

    #[test]
    fn new_rejects_unknown_key() {
        let err = WindowsListener::new("not-a-key").unwrap_err();
        assert!(matches!(err, HotkeyError::UnsupportedKey(_)));
    }

    #[test]
    fn double_start_is_rejected() {
        // 安装钩子会短暂占用系统资源，这里只验证逻辑分支
        // installing a hook is briefly resource-heavy; this only checks the branch
        let mut listener = WindowsListener::new("F9").unwrap();
        // 不真正 start（测试环境钩子会成功，但没必要）；手工放一个假 hook 状态
        // don't really start (the hook would succeed); fake the started state
        listener.hook = Some(HookHandle {
            thread_id: 0,
            thread: None,
        });
        let err = HotkeyListener::start(&mut listener).unwrap_err();
        assert!(matches!(err, HotkeyError::StartFailed(_)));
    }
}
