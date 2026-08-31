//! 识别历史：`history.json` 的追加 / 列出 / 删除 / 清空。
//!
//! Recognition history: append / list / delete / clear for `history.json`.
//!
//! 只保存 LaTeX 文本，不保存图片（与 altgo 只保存文本、不保存音频同一隐私立场）。
//! Only LaTeX text is stored, never images (same privacy stance as altgo: text only, no audio).

use crate::error::HistoryError;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// 历史条目（camelCase JSON）。
/// History entry (camelCase JSON).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub id: String,
    pub created_at_ms: u64,
    pub latex: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct HistoryFile {
    #[serde(default, rename = "entries")]
    entries: Vec<HistoryEntry>,
}

/// 全局文件 IO 互斥：进程内只有一个写者。
/// Global file-IO mutex: a single writer process-wide.
static HISTORY_IO_LOCK: Mutex<()> = Mutex::new(());

pub struct HistoryStore {
    path: PathBuf,
}

impl HistoryStore {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, HistoryError> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        if !path.exists() {
            std::fs::write(
                &path,
                serde_json::to_string(&HistoryFile::default()).unwrap(),
            )?;
        }
        Ok(Self { path })
    }

    /// 追加一条记录（新条目插头部）。
    /// Appends an entry (newest first).
    pub fn append(&self, latex: &str) -> Result<HistoryEntry, HistoryError> {
        if latex.trim().is_empty() {
            return Ok(HistoryEntry {
                id: String::new(),
                created_at_ms: 0,
                latex: String::new(),
            });
        }
        let entry = HistoryEntry {
            id: uuid::Uuid::new_v4().to_string(),
            created_at_ms: now_ms(),
            latex: latex.to_string(),
        };
        let _io = HISTORY_IO_LOCK.lock().unwrap();
        let mut file = self.read_file()?;
        file.entries.insert(0, entry.clone());
        self.write_file(&file)?;
        Ok(entry)
    }

    pub fn list(&self) -> Result<Vec<HistoryEntry>, HistoryError> {
        let _io = HISTORY_IO_LOCK.lock().unwrap();
        Ok(self.read_file()?.entries)
    }

    /// 删除指定 id 的条目，返回删除数量。
    /// Deletes entries by id; returns how many were removed.
    pub fn delete_entries(&self, ids: &[String]) -> Result<usize, HistoryError> {
        let _io = HISTORY_IO_LOCK.lock().unwrap();
        let mut file = self.read_file()?;
        let before = file.entries.len();
        file.entries.retain(|e| !ids.iter().any(|id| id == &e.id));
        let removed = before - file.entries.len();
        if removed > 0 {
            self.write_file(&file)?;
        }
        Ok(removed)
    }

    pub fn clear(&self) -> Result<(), HistoryError> {
        let _io = HISTORY_IO_LOCK.lock().unwrap();
        self.write_file(&HistoryFile::default())
    }

    fn read_file(&self) -> Result<HistoryFile, HistoryError> {
        let raw = std::fs::read_to_string(&self.path)?;
        serde_json::from_str(&raw).map_err(|e| HistoryError::Parse(e.to_string()))
    }

    fn write_file(&self, file: &HistoryFile) -> Result<(), HistoryError> {
        std::fs::write(&self.path, serde_json::to_string(file).unwrap())?;
        Ok(())
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store_in(dir: &Path) -> HistoryStore {
        HistoryStore::load(dir.join("history.json")).unwrap()
    }

    #[test]
    fn append_inserts_newest_first() {
        let dir = tempfile::tempdir().unwrap();
        let store = store_in(dir.path());
        store.append("a^2").unwrap();
        store.append("b_1").unwrap();
        let list = store.list().unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].latex, "b_1");
        assert_eq!(list[1].latex, "a^2");
    }

    #[test]
    fn empty_latex_is_not_stored() {
        let dir = tempfile::tempdir().unwrap();
        let store = store_in(dir.path());
        let e = store.append("   ").unwrap();
        assert!(e.id.is_empty());
        assert!(store.list().unwrap().is_empty());
    }

    #[test]
    fn delete_and_clear() {
        let dir = tempfile::tempdir().unwrap();
        let store = store_in(dir.path());
        let e1 = store.append("x").unwrap();
        store.append("y").unwrap();
        let removed = store.delete_entries(&[e1.id]).unwrap();
        assert_eq!(removed, 1);
        assert_eq!(store.list().unwrap().len(), 1);
        store.clear().unwrap();
        assert!(store.list().unwrap().is_empty());
    }

    #[test]
    fn entries_are_camel_case_json() {
        let dir = tempfile::tempdir().unwrap();
        let store = store_in(dir.path());
        store.append("z").unwrap();
        let raw = std::fs::read_to_string(dir.path().join("history.json")).unwrap();
        assert!(raw.contains("createdAtMs"), "JSON 键应为 camelCase");
        assert!(raw.contains("\"entries\""));
    }
}
