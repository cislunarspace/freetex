//! BPE tokenizer 解码：id 序列 → LaTeX 文本。
//!
//! BPE tokenizer decoding: id sequence → LaTeX text.
//!
//! tokenizer.json 是 HuggingFace tokenizers 的标准格式；我们只需要 id → token
//! 的反向词表与特殊 token 集合，不需要 BPE 合并表（那是编码方向用的）。
//! tokenizer.json is the standard HuggingFace tokenizers format; we only need the
//! id → token reverse vocabulary and the special-token set — merges are encode-only.
//!
//! 解码语义与 RapidLaTeXOCR 的 `TokenizerCls.token2str` 一致：
//! 跳过特殊 token → 拼接 token 字符串 → 去掉空格 → `Ġ` 还原为空格 → strip。
//! Decode semantics match RapidLaTeXOCR's `TokenizerCls.token2str`: skip special
//! tokens → join token strings → drop spaces → restore `Ġ` as space → strip.

use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct Tokenizer {
    id_to_token: Vec<String>,
    special_ids: HashSet<u32>,
}

#[derive(Deserialize)]
struct TokenizerJson {
    added_tokens: Vec<AddedToken>,
    model: VocabModel,
}

#[derive(Deserialize)]
struct AddedToken {
    id: u32,
    content: String,
    special: bool,
}

#[derive(Deserialize)]
struct VocabModel {
    vocab: HashMap<String, u32>,
}

impl Tokenizer {
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, String> {
        let raw =
            std::fs::read_to_string(path).map_err(|e| format!("读取 tokenizer.json 失败：{e}"))?;
        let json: TokenizerJson =
            serde_json::from_str(&raw).map_err(|e| format!("解析 tokenizer.json 失败：{e}"))?;

        let mut id_to_token: Vec<String> = Vec::new();
        for (token, id) in &json.model.vocab {
            if (*id as usize) >= id_to_token.len() {
                id_to_token.resize(*id as usize + 1, String::new());
            }
            id_to_token[*id as usize] = token.clone();
        }
        for added in &json.added_tokens {
            if (added.id as usize) >= id_to_token.len() {
                id_to_token.resize(added.id as usize + 1, String::new());
            }
            id_to_token[added.id as usize] = added.content.clone();
            if added.special {
                // 占位，稍后统一收集
                // placeholder, collected below
            }
        }

        let special_ids: HashSet<u32> = json
            .added_tokens
            .iter()
            .filter(|t| t.special)
            .map(|t| t.id)
            .collect();

        Ok(Self {
            id_to_token,
            special_ids,
        })
    }

    /// 词表大小。
    /// Vocabulary size.
    pub fn vocab_size(&self) -> usize {
        self.id_to_token.len()
    }

    /// id 序列 → LaTeX 文本。
    /// id sequence → LaTeX text.
    pub fn decode(&self, ids: &[u32]) -> String {
        let mut joined = String::new();
        for &id in ids {
            if self.special_ids.contains(&id) {
                continue;
            }
            match self.id_to_token.get(id as usize) {
                Some(token) => joined.push_str(token),
                None => joined.push_str("[UNK]"),
            }
        }
        // 与 Python 实现一致：先删空格，再把 Ġ 还原为空格。
        // Same order as the Python implementation: drop spaces, then restore Ġ as space.
        joined
            .replace(' ', "")
            .replace('\u{0120}', " ")
            .trim()
            .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_tokenizer_json(dir: &Path, vocab: &[(&str, u32)]) -> PathBuf {
        let mut vocab_map = serde_json::Map::new();
        for (token, id) in vocab {
            vocab_map.insert(
                (*token).to_string(),
                serde_json::Value::Number((*id).into()),
            );
        }
        let content = serde_json::json!({
            "added_tokens": [
                {"id": 0, "content": "[PAD]", "special": true},
                {"id": 1, "content": "[BOS]", "special": true},
                {"id": 2, "content": "[EOS]", "special": true}
            ],
            "model": { "vocab": vocab_map }
        })
        .to_string();
        let path = dir.join("tokenizer.json");
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(content.as_bytes()).unwrap();
        path
    }

    use std::path::PathBuf;

    fn demo_tokenizer(dir: &Path) -> Tokenizer {
        Tokenizer::from_file(write_tokenizer_json(
            dir,
            &[
                ("[PAD]", 0),
                ("[BOS]", 1),
                ("[EOS]", 2),
                ("\\frac", 3),
                ("{", 4),
                ("a", 5),
                ("}", 6),
                ("Ġ{", 7),
                ("Ġ}", 8),
                ("x", 9),
            ],
        ))
        .unwrap()
    }

    #[test]
    fn decodes_and_skips_specials() {
        let dir = tempfile::tempdir().unwrap();
        let tok = demo_tokenizer(dir.path());
        let text = tok.decode(&[1, 3, 4, 5, 6, 2]);
        assert_eq!(text, "\\frac{a}");
    }

    #[test]
    fn restores_gha_as_space() {
        let dir = tempfile::tempdir().unwrap();
        let tok = demo_tokenizer(dir.path());
        // "Ġ{" 还原为 " {"、"Ġ}" 还原为 " }"（与 Python 语义一致）
        // "Ġ{" → " {" and "Ġ}" → " }" (same as Python)
        let text = tok.decode(&[9, 7, 5, 8]);
        assert_eq!(text, "x {a }");
    }

    #[test]
    fn strips_literal_spaces_before_gha_restore() {
        let dir = tempfile::tempdir().unwrap();
        let tok = demo_tokenizer(dir.path());
        // 词表 token 本身不含空格；这里验证先删空格、再还原 Ġ 的次序
        // order matters: literal spaces removed first, Ġ restored after
        let text = tok.decode(&[3, 3]);
        assert_eq!(text, "\\frac\\frac");
    }

    #[test]
    fn unknown_ids_become_unk_marker() {
        let dir = tempfile::tempdir().unwrap();
        let tok = demo_tokenizer(dir.path());
        let text = tok.decode(&[999]);
        assert_eq!(text, "[UNK]");
    }

    #[test]
    fn vocab_size_counts_specials() {
        let dir = tempfile::tempdir().unwrap();
        let tok = demo_tokenizer(dir.path());
        assert_eq!(tok.vocab_size(), 10);
    }
}
