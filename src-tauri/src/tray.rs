//! 系统托盘：显示主窗 / 截图识别 / 退出。
//!
//! System tray: show main window / snip / quit.

use crate::snip::SnipManager;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager};

pub const MENU_SHOW: &str = "show";
pub const MENU_SNIP: &str = "snip";
pub const MENU_QUIT: &str = "quit";

/// 创建托盘；菜单事件直接驱动窗口与截图流程。
/// Creates the tray; menu events directly drive windows and the snip flow.
pub fn create_tray(app: &AppHandle) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, MENU_SHOW, "显示主窗口", true, None::<&str>)?;
    let snip = MenuItem::with_id(app, MENU_SNIP, "截图识别", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, MENU_QUIT, "退出", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &snip, &quit])?;

    let mut builder = TrayIconBuilder::with_id("freetex-tray")
        .tooltip("freetex — 公式识别")
        .menu(&menu)
        .show_menu_on_left_click(true);
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder
        .on_menu_event(|app, event| match event.id().as_ref() {
            MENU_SHOW => {
                if let Some(main) = app.get_webview_window("main") {
                    let _ = main.show();
                    let _ = main.set_focus();
                }
            }
            MENU_SNIP => {
                if let Some(manager) = app.try_state::<SnipManager>() {
                    let _ = manager.start();
                }
            }
            MENU_QUIT => app.exit(0),
            _ => {}
        })
        .build(app)?;
    Ok(())
}
