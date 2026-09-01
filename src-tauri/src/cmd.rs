//! Tauri 命令层：只做 IPC 参数转换，业务编排在各模块。
//!
//! The Tauri command layer: only IPC parameter conversion; orchestration lives
//! in the modules.
//!
//! 序列化契约：IPC 一律 camelCase（`ConfigDto`），TOML 文件仍由 `Config` 的
//! snake_case 承担。
//! Serialization contract: IPC is camelCase (`ConfigDto`); the TOML file stays
//! snake_case via `Config`.

use crate::config::{Config, SnipConfig};
use crate::config_store::ConfigStore;
use crate::engine::Recognizer;
use crate::error::FatalError;
use crate::history::{HistoryEntry, HistoryStore};
#[cfg(not(target_os = "android"))]
use crate::hotkey_manager::HotkeyManager;
use crate::model;
use crate::pipeline::Job;
use crate::pipeline_controller::PipelineController;
#[cfg(not(target_os = "android"))]
use crate::snip::{SnipManager, SnipRect};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, State};

// ---------- 配置 ----------

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SnipDto {
    pub hotkey: String,
    pub auto_copy: bool,
    pub copy_format: String,
    pub hide_main_during_snip: bool,
}

impl Default for SnipDto {
    fn default() -> Self {
        let c = SnipConfig::default();
        Self {
            hotkey: c.hotkey,
            auto_copy: c.auto_copy,
            copy_format: c.copy_format,
            hide_main_during_snip: c.hide_main_during_snip,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct EngineDto {
    pub model: String,
    pub num_threads: usize,
}

impl Default for EngineDto {
    fn default() -> Self {
        let c = crate::config::EngineConfig::default();
        Self {
            model: c.model,
            num_threads: c.num_threads,
        }
    }
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ConfigDto {
    pub snip: SnipDto,
    pub engine: EngineDto,
}

impl From<Config> for ConfigDto {
    fn from(c: Config) -> Self {
        Self {
            snip: SnipDto {
                hotkey: c.snip.hotkey,
                auto_copy: c.snip.auto_copy,
                copy_format: c.snip.copy_format,
                hide_main_during_snip: c.snip.hide_main_during_snip,
            },
            engine: EngineDto {
                model: c.engine.model,
                num_threads: c.engine.num_threads,
            },
        }
    }
}

impl ConfigDto {
    fn apply_to(&self, config: &mut Config) {
        config.snip.hotkey = self.snip.hotkey.clone();
        config.snip.auto_copy = self.snip.auto_copy;
        config.snip.copy_format = self.snip.copy_format.clone();
        config.snip.hide_main_during_snip = self.snip.hide_main_during_snip;
        config.engine.model = self.engine.model.clone();
        config.engine.num_threads = self.engine.num_threads;
    }
}

#[tauri::command]
pub fn get_config(store: State<'_, Arc<ConfigStore>>) -> Result<ConfigDto, String> {
    Ok(store.get().into())
}

/// 应用补丁并返回（旧配置，新配置）；桌面壳再做快捷键重建。
/// Applies the patch and returns (old, new); the desktop wrapper also rebuilds hotkeys.
fn apply_config_patch(
    store: &State<'_, Arc<ConfigStore>>,
    patch: &ConfigDto,
) -> Result<(Config, Config), String> {
    let previous = store.get();
    let updated: Config = store
        .apply_patch(&|config| patch.apply_to(config))
        .map_err(|e| e.to_string())?;
    Ok((previous, updated))
}

#[cfg(not(target_os = "android"))]
#[tauri::command]
pub fn save_config(
    app: AppHandle,
    store: State<'_, Arc<ConfigStore>>,
    hotkey: State<'_, HotkeyManager>,
    patch: ConfigDto,
) -> Result<ConfigDto, String> {
    let (previous, updated) = apply_config_patch(&store, &patch)?;

    // 快捷键变化重建监听器；其余配置重启流水线（引擎懒加载，代价低）
    // rebuild the listener on hotkey change; restart the pipeline otherwise
    // (the lazy engine keeps restarts cheap)
    if updated.snip.hotkey != previous.snip.hotkey {
        hotkey.restart(&app, &updated.snip.hotkey);
    }
    restart_pipeline(&app, &store);

    Ok(updated.into())
}

/// Android 无全局快捷键，save_config 只需落盘 + 重启流水线。
/// Android has no global hotkeys; save_config only persists and restarts the pipeline.
#[cfg(target_os = "android")]
#[tauri::command]
pub fn save_config(
    app: AppHandle,
    store: State<'_, Arc<ConfigStore>>,
    patch: ConfigDto,
) -> Result<ConfigDto, String> {
    let (_previous, updated) = apply_config_patch(&store, &patch)?;
    restart_pipeline(&app, &store);
    Ok(updated.into())
}

// ---------- 截图与识别 ----------

#[cfg(not(target_os = "android"))]
#[tauri::command]
pub fn start_snip(snip: State<'_, SnipManager>) -> Result<(), String> {
    snip.start()
}

#[cfg(not(target_os = "android"))]
#[tauri::command]
pub fn cancel_snip(snip: State<'_, SnipManager>) -> Result<(), String> {
    snip.cancel();
    Ok(())
}

#[cfg(not(target_os = "android"))]
#[tauri::command]
pub fn confirm_snip(snip: State<'_, SnipManager>, rect: SnipRect) -> Result<(), String> {
    snip.confirm(rect)
}

/// Android 桩：选区截图链路不存在，前端改走选图。
/// Android stubs: the snip flow doesn't exist; the frontend picks images instead.
#[cfg(target_os = "android")]
#[tauri::command]
pub fn start_snip() -> Result<(), String> {
    Err("移动端不支持截图选区，请选择图片识别".to_string())
}

#[cfg(target_os = "android")]
#[tauri::command]
pub fn cancel_snip() -> Result<(), String> {
    Err("移动端不支持截图选区，请选择图片识别".to_string())
}

#[cfg(target_os = "android")]
#[tauri::command]
pub fn confirm_snip(_rect: serde_json::Value) -> Result<(), String> {
    Err("移动端不支持截图选区，请选择图片识别".to_string())
}

#[tauri::command]
pub fn recognize_image_bytes(
    controller: State<'_, PipelineController>,
    bytes: Vec<u8>,
) -> Result<(), String> {
    if bytes.is_empty() {
        return Err("图片内容为空".to_string());
    }
    controller
        .submit(Job::ImageBytes { bytes })
        .then_some(())
        .ok_or_else(|| "识别流水线未运行".to_string())
}

#[tauri::command]
pub fn recognize_image_path(
    controller: State<'_, PipelineController>,
    path: String,
) -> Result<(), String> {
    let bytes = std::fs::read(&path).map_err(|e| format!("读取图片失败：{e}"))?;
    controller
        .submit(Job::ImageBytes { bytes })
        .then_some(())
        .ok_or_else(|| "识别流水线未运行".to_string())
}

#[tauri::command]
pub fn copy_text(app: AppHandle, text: String) -> Result<(), String> {
    use crate::clipboard::{Clipboard, PlatformClipboard};
    PlatformClipboard::new(app)
        .copy_text(&text)
        .map_err(|e| e.to_string())
}

/// 复制 MathML：以 `text/html` 剪贴板格式承载（Word 粘贴即公式），纯文本兜底。
/// Copies MathML via the `text/html` clipboard format (Word pastes it as an
/// equation), with plain text as the fallback.
#[tauri::command]
pub fn copy_mathml(app: AppHandle, mathml: String, plain: String) -> Result<(), String> {
    use crate::clipboard::{Clipboard, PlatformClipboard};
    let html =
        format!(r#"<html><body><!--StartFragment-->{mathml}<!--EndFragment--></body></html>"#);
    PlatformClipboard::new(app)
        .copy_html(&html, &plain)
        .map_err(|e| e.to_string())
}

// ---------- 模型管理 ----------

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelFileDto {
    pub name: String,
    pub ready: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelInfoDto {
    pub name: String,
    pub dir: String,
    pub ready: bool,
    pub files: Vec<ModelFileDto>,
}

#[tauri::command]
pub fn list_models(store: State<'_, Arc<ConfigStore>>) -> Result<Vec<ModelInfoDto>, String> {
    let configured = store.get().engine.model;
    let dir = model::resolve_model_dir(&configured);
    Ok(vec![ModelInfoDto {
        name: model::MODEL_NAME.to_string(),
        dir: dir.display().to_string(),
        ready: model::model_ready(&dir),
        files: model::MODEL_FILES
            .iter()
            .map(|f| ModelFileDto {
                name: f.name.to_string(),
                ready: model::file_ready(&dir, f),
            })
            .collect(),
    }])
}

#[tauri::command]
pub fn download_model(app: AppHandle, name: String) -> Result<(), String> {
    if name != model::MODEL_NAME {
        return Err(format!("不支持的模型：{name}"));
    }
    let dir = model::resolve_model_dir(&name);
    // 后台线程阻塞下载，进度经事件上报（命令立即返回）
    // background thread downloads blocking; progress flows through events
    // (the command returns immediately)
    std::thread::Builder::new()
        .name("freetex-model-download".into())
        .spawn(move || {
            let mut progress = |p: &model::DownloadProgress| {
                let _ = app.emit(
                    "model-download-progress",
                    serde_json::json!({
                        "fileName": p.file,
                        "downloaded": p.downloaded,
                        "total": p.total,
                        "fileIndex": p.file_index,
                        "fileCount": p.file_count,
                        "overallDownloaded": p.overall_downloaded,
                        "overallTotal": p.overall_total,
                        "source": p.source,
                    }),
                );
            };
            let result = model::download_model(&dir, &mut progress);
            let payload = match &result {
                Ok(()) => serde_json::json!({ "success": true }),
                Err(err) => serde_json::json!({ "success": false, "message": err.to_string() }),
            };
            let _ = app.emit("model-download-finished", payload);
            if let Err(err) = result {
                tracing::error!(error = %err, "模型下载失败");
            }
        })
        .map_err(|e| format!("下载线程启动失败：{e}"))?;
    Ok(())
}

#[tauri::command]
pub fn delete_model(name: String) -> Result<(), String> {
    if name != model::MODEL_NAME {
        return Err(format!("不支持的模型：{name}"));
    }
    let dir = model::resolve_model_dir(&name);
    if dir.exists() {
        std::fs::remove_dir_all(&dir).map_err(|e| format!("删除模型失败：{e}"))?;
    }
    Ok(())
}

#[tauri::command]
pub fn resolve_model(store: State<'_, Arc<ConfigStore>>) -> Result<serde_json::Value, String> {
    let configured = store.get().engine.model;
    let dir = model::resolve_model_dir(&configured);
    Ok(serde_json::json!({
        "path": dir.display().to_string(),
        "ready": model::model_ready(&dir),
    }))
}

// ---------- 历史 ----------

#[tauri::command]
pub fn list_history(history: State<'_, Arc<HistoryStore>>) -> Result<Vec<HistoryEntry>, String> {
    history.list().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_history_entries(
    history: State<'_, Arc<HistoryStore>>,
    ids: Vec<String>,
) -> Result<usize, String> {
    history.delete_entries(&ids).map_err(|e| e_to_string(&e))
}

#[tauri::command]
pub fn clear_history(history: State<'_, Arc<HistoryStore>>) -> Result<(), String> {
    history.clear().map_err(|e| e_to_string(&e))
}

fn e_to_string(e: &crate::error::HistoryError) -> String {
    e.to_string()
}

// ---------- 应用更新（仅桌面；Android 走应用商店 / 直接下载 APK） ----------

/// 检查更新：静默模式（启动时）失败静默，手动模式带 10 秒超时与分类错误。
/// Checks for updates: silent mode (startup) fails quietly; manual mode carries a
/// 10s timeout and categorized errors. `mode` 保留给调用方语义区分，核心行为一致。
#[cfg(not(target_os = "android"))]
#[tauri::command]
pub async fn check_update(
    app: AppHandle,
    mode: crate::updater::CheckMode,
) -> Result<crate::updater::UpdateCheckResponse, crate::updater::UpdateErrorResponse> {
    let _ = mode;
    let provider = crate::updater::TauriUpdateProvider { app };
    let support_tier = crate::updater::detect_support_tier();
    crate::updater::check_update_core(&provider, std::time::Duration::from_secs(10), support_tier)
        .await
}

/// 下载并安装更新（就地更新；识别进行中拒绝）。
/// Downloads and installs the update (in place; refused while recognizing).
#[cfg(not(target_os = "android"))]
#[tauri::command]
pub async fn install_update(
    app: AppHandle,
    controller: State<'_, PipelineController>,
) -> Result<(), String> {
    let provider = crate::updater::TauriUpdateProvider { app };
    crate::updater::install_update_core(&provider, &controller).await
}

/// 从 updater endpoint 推导发布页地址（外部引导更新用）。
/// Derives the releases page URL from the updater endpoint (for external-tier updates).
#[cfg(not(target_os = "android"))]
#[tauri::command]
pub fn get_releases_url(app: AppHandle) -> String {
    app.config()
        .plugins
        .0
        .get("updater")
        .and_then(|u| u.get("endpoints"))
        .and_then(|e| e.as_array())
        .and_then(|list| list.first())
        .and_then(|url| url.as_str())
        .and_then(|url| {
            // .../releases/latest/download/latest.json → .../releases/latest
            url.strip_suffix("latest/download/latest.json")
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| "https://github.com/your-name/freetex/releases".to_string())
}

/// Android 桩：更新走发布页直接下载 APK，无应用内更新。
/// Android stubs: updates download APKs from the releases page, not in-app.
#[cfg(target_os = "android")]
#[tauri::command]
pub async fn check_update() -> Result<serde_json::Value, String> {
    Err("移动端请到发布页下载新版 APK".to_string())
}

#[cfg(target_os = "android")]
#[tauri::command]
pub async fn install_update() -> Result<(), String> {
    Err("移动端请到发布页下载新版 APK".to_string())
}

#[cfg(target_os = "android")]
#[tauri::command]
pub fn get_releases_url() -> String {
    "https://github.com/cislunarspace/freetex/releases".to_string()
}

// ---------- 流水线装配 ----------

/// 用当前配置重建流水线（save_config / 启动时共用）。
/// Rebuilds the pipeline from current config (shared by save_config / startup).
pub fn restart_pipeline(app: &AppHandle, store: &State<'_, Arc<ConfigStore>>) {
    let controller = app.state::<PipelineController>();
    let history = app.state::<Arc<HistoryStore>>();
    let config_store = store.inner().clone();

    let deps = crate::pipeline::PipelineDeps {
        sink: Box::new(crate::tauri_sink::TauriSink::new(
            app.clone(),
            controller.status_arc(),
        )),
        clipboard: Arc::new(crate::clipboard::PlatformClipboard::new(app.clone())),
        history: history.inner().clone(),
        engine_factory: Box::new(move || {
            let config = config_store.get();
            let dir = model::resolve_model_dir(&config.engine.model);
            crate::engine::load_engine(&dir, config.engine.num_threads)
                .map(|engine| Box::new(engine) as Box<dyn Recognizer>)
                .map_err(|e| match e {
                    crate::error::EngineError::ModelMissing(p) => FatalError::EngineInit(format!(
                        "模型文件缺失：{p}。请到「设置 → 模型」下载后重试"
                    )),
                    other => FatalError::EngineInit(other.to_string()),
                })
        }),
        auto_copy: store.get().snip.auto_copy,
        copy_format: crate::config::CopyFormat::parse(&store.get().snip.copy_format),
        capture_settle: std::time::Duration::from_millis(150),
    };
    controller.start(deps);
}
