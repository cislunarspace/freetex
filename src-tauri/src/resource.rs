//! 路径与线程数工具。
//!
//! Path and thread-count helpers.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// 展开 `~` 为用户主目录；其余原样返回。
/// Expands `~` to the home directory; other paths pass through unchanged.
pub fn expand_tilde(path: &str) -> PathBuf {
    if path == "~" {
        if let Some(home) = dirs::home_dir() {
            return home;
        }
    } else if let Some(rest) = path.strip_prefix("~/").or_else(|| path.strip_prefix("~\\")) {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    }
    PathBuf::from(path)
}

/// 解析线程数：0 表示自动取满核；显式值不超过核数上限。
/// Resolves thread count: 0 means "use all cores"; explicit values cap at cores.
pub fn effective_threads(configured: usize) -> usize {
    if configured == 0 {
        available_cores()
    } else {
        configured.min(available_cores())
    }
}

/// 可用核数。
/// Available core count.
pub fn available_cores() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
}

/// Android 上 `dirs::config_dir()` 不可用，由 lib.rs setup 用 Tauri 路径解析器
/// 注入；桌面不注入，保持既有路径不变。
/// On Android `dirs::config_dir()` is unavailable, so lib.rs setup injects the
/// dir via the Tauri path resolver; desktop skips injection to keep paths stable.
static ANDROID_DATA_DIR: OnceLock<PathBuf> = OnceLock::new();

/// 注入 Android 数据目录（仅 Android 启动时调用一次）。
/// Injects the Android data dir (called once at startup on Android only).
pub fn set_data_dir(dir: PathBuf) {
    let _ = ANDROID_DATA_DIR.set(dir);
}

/// 应用数据目录（配置、历史、模型的根）。
/// App data directory (root for config, history, and models).
pub fn app_data_dir() -> PathBuf {
    if let Some(dir) = ANDROID_DATA_DIR.get() {
        return dir.clone();
    }
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("freetex")
}

/// 检查文件是否存在且大小不低于 `min_size`（识别损坏缓存）。
/// Checks a file exists and is at least `min_size` bytes (catches corrupt caches).
pub fn file_ready(path: &Path, min_size: u64) -> bool {
    std::fs::metadata(path)
        .map(|m| m.is_file() && m.len() >= min_size)
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expand_tilde_resolves_home() {
        let p = expand_tilde("~/freetex.toml");
        assert!(!p.starts_with("~"), "应已展开 ~ / ~ should be expanded");
        assert!(p.ends_with("freetex.toml"));
    }

    #[test]
    fn expand_tilde_keeps_plain_paths() {
        assert_eq!(
            expand_tilde("/etc/freetex.toml"),
            PathBuf::from("/etc/freetex.toml")
        );
    }

    #[test]
    fn effective_threads_clamps() {
        assert_eq!(effective_threads(0), available_cores());
        assert_eq!(effective_threads(2), 2);
    }
}
