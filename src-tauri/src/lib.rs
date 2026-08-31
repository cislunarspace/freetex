//! freetex 库入口：模块声明与 Tauri 装配。
//!
//! freetex library root: module declarations and Tauri assembly.

pub mod capture;
pub mod clipboard;
pub mod cmd;
pub mod config;
pub mod config_store;
pub mod engine;
pub mod error;
pub mod history;
pub mod hotkey;
pub mod hotkey_manager;
pub mod model;
pub mod pipeline;
pub mod pipeline_controller;
pub mod resource;
pub mod snip;
pub mod tauri_sink;
pub mod tray;
pub mod updater;

use config_store::ConfigStore;
use history::HistoryStore;
use hotkey_manager::HotkeyManager;
use pipeline_controller::PipelineController;
use snip::SnipManager;
use std::sync::Arc;
use tauri::{Manager, WindowEvent};

/// 装配并运行 Tauri 应用。
/// Assembles and runs the Tauri app.
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // 二次启动：唤起已有主窗口
            // second launch: raise the existing main window
            if let Some(main) = app.get_webview_window("main") {
                let _ = main.show();
                let _ = main.set_focus();
            }
        }))
        .setup(|app| {
            let data_dir = resource::app_data_dir();
            let store = Arc::new(ConfigStore::load(data_dir.join("freetex.toml"))?);
            let history = Arc::new(HistoryStore::load(data_dir.join("history.json"))?);
            app.manage(store.clone());
            app.manage(history.clone());
            app.manage(PipelineController::default());
            app.manage(HotkeyManager::default());
            app.manage(SnipManager::new(app.handle().clone()));

            tray::create_tray(app.handle())?;

            let handle = app.handle().clone();
            let hotkey = handle.state::<HotkeyManager>();
            hotkey.start_from_config(&handle, &store);
            cmd::restart_pipeline(&handle, &handle.state::<Arc<ConfigStore>>());

            // 关窗 = 隐藏到托盘（与 altgo 同策略）
            // closing = hide to tray (same policy as altgo)
            if let Some(main) = app.get_webview_window("main") {
                let handle = app.handle().clone();
                main.on_window_event(move |event| {
                    if let WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        if let Some(main) = handle.get_webview_window("main") {
                            let _ = main.hide();
                        }
                    }
                });
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            cmd::get_config,
            cmd::save_config,
            cmd::start_snip,
            cmd::cancel_snip,
            cmd::confirm_snip,
            cmd::recognize_image_bytes,
            cmd::recognize_image_path,
            cmd::copy_text,
            cmd::copy_mathml,
            cmd::list_models,
            cmd::download_model,
            cmd::delete_model,
            cmd::resolve_model,
            cmd::list_history,
            cmd::delete_history_entries,
            cmd::clear_history,
            cmd::check_update,
            cmd::install_update,
            cmd::get_releases_url,
        ])
        .run(tauri::generate_context!())
        .expect("freetex 启动失败");
}
