//! freetex 库入口：模块声明与 Tauri 装配。
//!
//! freetex library root: module declarations and Tauri assembly.
//!
//! Android 上不存在的能力（托盘、全局快捷键、选区截图、单实例、应用内更新）
//! 整体不编译，装配按平台分叉；识别链路（引擎 / 流水线 / 历史）两端共用。
//! Capabilities that don't exist on Android (tray, global hotkeys, snip,
//! single instance, in-app updates) are compiled out; the recognition chain
//! (engine / pipeline / history) stays shared.

pub mod capture;
pub mod clipboard;
pub mod cmd;
pub mod config;
pub mod config_store;
pub mod engine;
pub mod error;
pub mod history;
#[cfg(not(target_os = "android"))]
pub mod hotkey;
#[cfg(not(target_os = "android"))]
pub mod hotkey_manager;
pub mod model;
pub mod pipeline;
pub mod pipeline_controller;
pub mod resource;
#[cfg(not(target_os = "android"))]
pub mod snip;
pub mod tauri_sink;
#[cfg(not(target_os = "android"))]
pub mod tray;
#[cfg(not(target_os = "android"))]
pub mod updater;

use config_store::ConfigStore;
use history::HistoryStore;
use pipeline_controller::PipelineController;
use std::sync::Arc;
use tauri::Manager;

/// 装配并运行 Tauri 应用。
/// Assembles and runs the Tauri app.
pub fn run() {
    let builder = tauri::Builder::default().plugin(tauri_plugin_dialog::init());

    // Android：arboard 不可用，剪贴板走官方插件（JNI 桥接）
    // Android: arboard is unavailable; clipboard goes through the official plugin (JNI)
    #[cfg(target_os = "android")]
    let builder = builder.plugin(tauri_plugin_clipboard_manager::init());

    // 桌面专属：单实例（二次启动唤起主窗）与应用内更新
    // Desktop-only: single instance (second launch raises the main window) and in-app updates
    #[cfg(not(target_os = "android"))]
    let builder = builder
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(main) = app.get_webview_window("main") {
                let _ = main.show();
                let _ = main.set_focus();
            }
        }));

    builder
        .setup(|app| {
            // Android 上 dirs::config_dir() 不可用，先注入 Tauri 解析出的数据目录
            // dirs::config_dir() is unavailable on Android; inject the Tauri-resolved dir first
            #[cfg(target_os = "android")]
            resource::set_data_dir(
                app.path()
                    .app_data_dir()
                    .map_err(|e| format!("解析数据目录失败：{e}"))?
                    .join("freetex"),
            );

            let data_dir = resource::app_data_dir();
            let store = Arc::new(ConfigStore::load(data_dir.join("freetex.toml"))?);
            let history = Arc::new(HistoryStore::load(data_dir.join("history.json"))?);
            app.manage(store.clone());
            app.manage(history.clone());
            app.manage(PipelineController::default());

            // 桌面专属装配：托盘、快捷键、选区窗、关窗驻留托盘（与 altgo 同策略）
            // Desktop-only assembly: tray, hotkeys, snip window, close-to-tray
            // (same policy as altgo)
            #[cfg(not(target_os = "android"))]
            {
                app.manage(hotkey_manager::HotkeyManager::default());
                app.manage(snip::SnipManager::new(app.handle().clone()));

                tray::create_tray(app.handle())?;

                let handle = app.handle().clone();
                let hotkey = handle.state::<hotkey_manager::HotkeyManager>();
                hotkey.start_from_config(&handle, &store);

                if let Some(main) = app.get_webview_window("main") {
                    let handle = app.handle().clone();
                    main.on_window_event(move |event| {
                        if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                            api.prevent_close();
                            if let Some(main) = handle.get_webview_window("main") {
                                let _ = main.hide();
                            }
                        }
                    });
                }
            }

            cmd::restart_pipeline(app.handle(), &app.state::<Arc<ConfigStore>>());
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
