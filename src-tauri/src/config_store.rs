//! `Config` 的持久化封装：读、补丁校验、写盘。
//!
//! Persistence wrapper for `Config`: load, validated patching, and saving.
//!
//! 校验失败时不落盘；写盘前持锁，避免并发保存交错。
//! Failed validation never writes; a lock is held across validate-and-write so
//! concurrent saves cannot interleave.

use crate::config::Config;
use crate::error::ConfigError;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

pub struct ConfigStore {
    path: PathBuf,
    config: Mutex<Config>,
}

impl ConfigStore {
    /// 从 `path` 加载；文件缺失时使用默认配置（不立即写盘）。
    /// Loads from `path`; a missing file falls back to defaults (nothing written yet).
    pub fn load(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let path = path.as_ref().to_path_buf();
        let config = if path.exists() {
            let raw = std::fs::read_to_string(&path)?;
            toml::from_str(&raw).map_err(|e| ConfigError::Parse(e.to_string()))?
        } else {
            Config::default()
        };
        Ok(Self {
            path,
            config: Mutex::new(config),
        })
    }

    /// 当前配置快照。
    /// Snapshot of the current config.
    pub fn get(&self) -> Config {
        self.config.lock().unwrap().clone()
    }

    /// 用补丁更新配置：校验通过后写盘。返回更新后的配置。
    /// Applies a patch: validates, then writes. Returns the updated config.
    pub fn apply_patch(&self, patch: &dyn Fn(&mut Config)) -> Result<Config, ConfigError> {
        let mut guard = self.config.lock().unwrap();
        let mut candidate = guard.clone();
        patch(&mut candidate);
        candidate.validate()?;
        self.write(&candidate)?;
        *guard = candidate;
        Ok(guard.clone())
    }

    fn write(&self, config: &Config) -> Result<(), ConfigError> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let raw = toml::to_string_pretty(config).map_err(|e| ConfigError::Parse(e.to_string()))?;
        std::fs::write(&self.path, raw)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::SnipConfig;

    fn store_in(dir: &Path) -> ConfigStore {
        ConfigStore::load(dir.join("freetex.toml")).unwrap()
    }

    #[test]
    fn missing_file_yields_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let store = store_in(dir.path());
        assert_eq!(store.get(), Config::default());
    }

    #[test]
    fn apply_patch_writes_file_and_reloads() {
        let dir = tempfile::tempdir().unwrap();
        let store = store_in(dir.path());
        store
            .apply_patch(&|cfg| cfg.snip.hotkey = "Insert".to_string())
            .unwrap();

        let reloaded = ConfigStore::load(dir.path().join("freetex.toml")).unwrap();
        assert_eq!(reloaded.get().snip.hotkey, "Insert");
    }

    #[test]
    fn invalid_patch_rejected_and_not_written() {
        let dir = tempfile::tempdir().unwrap();
        let store = store_in(dir.path());
        let err = store
            .apply_patch(&|cfg: &mut Config| {
                cfg.snip = SnipConfig {
                    hotkey: "NotAKey".to_string(),
                    ..SnipConfig::default()
                }
            })
            .unwrap_err();
        assert!(matches!(err, ConfigError::Validation(_)));
        assert_eq!(store.get().snip.hotkey, "F9", "内存配置不应被污染");
        assert!(
            !dir.path().join("freetex.toml").exists(),
            "校验失败不应落盘"
        );
    }
}
