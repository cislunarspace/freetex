//! 模型管理：注册、下载、校验、解析（移植自 altgo 的 model.rs 模式）。
//!
//! Model management: registry, download, verification, resolution (ported from
//! altgo's model.rs pattern).
//!
//! - 唯一内置模型 `latex-ocr`（RapidAI/RapidLaTeXOCR v0.0.0 转换的 LaTeX-OCR ONNX）。
//! - 双源回退：环境变量 `FREETEX_MODEL_BASE_URL` 优先，官方 GitHub Releases 兜底。
//! - 每个文件内置 SHA-256 与最小体积；下载写 `.tmp`，校验通过后原子改名。
//! - 已完整（大小 + 哈希通过）的文件直接跳过下载。
//!
//! English summary: single built-in model `latex-ocr`; dual-source fallback
//! (`FREETEX_MODEL_BASE_URL` env first, official GitHub Releases second); per-file
//! SHA-256 + minimum size verification; downloads go through a `.tmp` file with an
//! atomic rename; complete files are skipped.

use crate::error::ModelError;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// 官方下载基址（GitHub Releases，tag v0.0.0）。
/// Official download base (GitHub Releases, tag v0.0.0).
pub const OFFICIAL_BASE_URL: &str =
    "https://github.com/RapidAI/RapidLaTeXOCR/releases/download/v0.0.0";

/// 内置镜像基址（公共 gh 代理，国内可达；可用性可能波动，失败自动回退下一个源）。
/// 前缀 + 完整 GitHub URL 的拼接形式，与 `base_urls()` 的 `base/{file}` 拼接兼容。
/// Built-in mirrors (public gh proxies reachable from mainland China; availability
/// may vary, failures fall through to the next base). Prefix + full GitHub URL form,
/// compatible with the `base/{file}` join in `base_urls()`.
pub const MIRROR_BASE_URLS: &[&str] = &[
    "https://ghfast.top/https://github.com/RapidAI/RapidLaTeXOCR/releases/download/v0.0.0",
    "https://gh-proxy.com/https://github.com/RapidAI/RapidLaTeXOCR/releases/download/v0.0.0",
];

/// 环境变量：覆盖模型下载基址（镜像优先）。
/// Env var: overrides the model download base (mirror first).
pub const MODEL_BASE_URL_ENV: &str = "FREETEX_MODEL_BASE_URL";

/// 唯一内置模型名。
/// The only built-in model name.
pub const MODEL_NAME: &str = "latex-ocr";

#[derive(Debug, Clone)]
pub struct ModelFileSpec {
    pub name: &'static str,
    pub sha256: &'static str,
    pub min_size: u64,
    /// 官方 Release 实测确切大小：用于跨文件总体进度，不依赖网络。
    /// Exact size measured from the official Release; powers overall progress
    /// without depending on the network.
    pub size: u64,
}

/// latex-ocr 的四个文件（哈希来自官方 Release 实测）。
/// The four latex-ocr files (hashes measured from the official Release).
pub const MODEL_FILES: &[ModelFileSpec] = &[
    ModelFileSpec {
        name: "encoder.onnx",
        sha256: "01bf5dc25539ca0cd5b1bd29296ea495977a6ba5f629dc4178277809d26e5e7d",
        min_size: 10_000_000,
        size: 89008136,
    },
    ModelFileSpec {
        name: "decoder.onnx",
        sha256: "bd695497bf1b22279b7626f5916c79226e1e244c84355f8da7edfd2d921d0072",
        min_size: 10_000_000,
        size: 50952726,
    },
    ModelFileSpec {
        name: "image_resizer.onnx",
        sha256: "e0b075c39700f64d50400f39c8fc186bbb3b5d84d31864008313f376603aca9d",
        min_size: 5_000_000,
        size: 38967751,
    },
    ModelFileSpec {
        name: "tokenizer.json",
        sha256: "1dc27b18d6a518d0d5ff3f4bb7bd98521fe80ad39e5b2a246d4109f1bb9d5019",
        min_size: 1_000,
        size: 24174,
    },
];

/// 下载进度快照：单文件 + 跨文件总体，source 为当前源 host。
/// Download progress snapshot: per-file plus overall; `source` is the current host.
pub struct DownloadProgress {
    pub file: String,
    /// 1 起始的文件序号。
    /// 1-based file index.
    pub file_index: usize,
    pub file_count: usize,
    /// 当前文件已下载 / 总字节（总字节未知时为 0）。
    /// Current file downloaded / total bytes (total 0 when unknown).
    pub downloaded: u64,
    pub total: u64,
    /// 全部文件累计已下载 / 总字节（总字节来自内置 size，不依赖网络）。
    /// Overall downloaded / total bytes (totals come from built-in sizes, no network).
    pub overall_downloaded: u64,
    pub overall_total: u64,
    pub source: String,
}

/// 下载进度回调（按快照上报）。
/// Download progress callback (reports snapshots).
pub type ProgressFn<'a> = dyn FnMut(&DownloadProgress) + 'a;

/// 下载基址列表：env 覆盖优先，其次内置镜像（国内可达），官方兜底。
/// Download base URLs: env override first, then built-in mirrors (mainland
/// China reachable), official last.
pub fn base_urls() -> Vec<String> {
    let mut urls = Vec::new();
    if let Ok(custom) = std::env::var(MODEL_BASE_URL_ENV) {
        let custom = custom.trim_end_matches('/').to_string();
        if !custom.is_empty() {
            urls.push(custom);
        }
    }
    urls.extend(MIRROR_BASE_URLS.iter().map(|s| s.to_string()));
    urls.push(OFFICIAL_BASE_URL.to_string());
    urls
}

/// 默认模型根目录：`<config>/freetex/models/`。
/// Default models root: `<config>/freetex/models/`.
pub fn default_models_dir() -> PathBuf {
    crate::resource::app_data_dir().join("models")
}

/// 解析配置里的模型值：内置名 → 默认目录；目录/文件路径原样（自定义模型只做结构校验）。
/// Resolves the configured model value: built-in name → default dir; a directory or
/// file path passes through (custom models only get structural checks).
pub fn resolve_model_dir(model_value: &str) -> PathBuf {
    if model_value == MODEL_NAME || model_value.is_empty() {
        return default_models_dir().join(MODEL_NAME);
    }
    crate::resource::expand_tilde(model_value)
}

/// 模型是否就绪（全部文件存在且校验通过；自定义目录只要求文件存在）。
/// Whether the model is ready (all files verified; custom dirs only need presence).
pub fn model_ready(model_dir: &Path) -> bool {
    MODEL_FILES.iter().all(|f| {
        let path = model_dir.join(f.name);
        if !crate::resource::file_ready(&path, f.min_size) {
            return false;
        }
        // 默认模型目录做完整哈希校验；用户自定义目录只查存在性
        // full hash verification for the default dir; presence-only for custom dirs
        match model_dir.ends_with(MODEL_NAME) {
            true => file_sha256(&path).map(|h| h == f.sha256).unwrap_or(false),
            false => true,
        }
    })
}

/// 单文件是否已就绪（大小 + SHA-256）。
/// Whether one file is complete (size + SHA-256).
pub fn file_ready(model_dir: &Path, spec: &ModelFileSpec) -> bool {
    let path = model_dir.join(spec.name);
    crate::resource::file_ready(&path, spec.min_size)
        && file_sha256(&path)
            .map(|h| h == spec.sha256)
            .unwrap_or(false)
}

/// 计算文件 SHA-256（小写 hex）。
/// Computes a file's SHA-256 (lowercase hex).
pub fn file_sha256(path: &Path) -> Result<String, ModelError> {
    let data = std::fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(&data);
    let digest = hasher.finalize();
    Ok(digest.iter().map(|b| format!("{b:02x}")).collect())
}

/// 下载模型（已完整的文件跳过），进度经回调上报。
/// Downloads the model (complete files are skipped), reporting progress via callback.
pub fn download_model(model_dir: &Path, on_progress: &mut ProgressFn) -> Result<(), ModelError> {
    std::fs::create_dir_all(model_dir)?;
    let bases = base_urls();
    let client = reqwest::blocking::Client::builder()
        // 连接 15 秒建不起来就换下一个源，避免在不可达源上干等
        // give up connecting after 15s and move on to the next source
        .connect_timeout(std::time::Duration::from_secs(15))
        .timeout(std::time::Duration::from_secs(600))
        .build()
        .map_err(|e| ModelError::Download(e.to_string()))?;

    let file_count = MODEL_FILES.len();
    let overall_total: u64 = MODEL_FILES.iter().map(|f| f.size).sum();
    let mut done_bytes: u64 = 0;

    for (index, spec) in MODEL_FILES.iter().enumerate() {
        if file_ready(model_dir, spec) {
            tracing::info!(file = spec.name, "模型文件已就绪，跳过下载");
            continue;
        }
        let file_index = index + 1;
        let mut on_file_progress = |downloaded: u64, total: u64, source: &str| {
            on_progress(&DownloadProgress {
                file: spec.name.to_string(),
                file_index,
                file_count,
                downloaded,
                total,
                overall_downloaded: done_bytes + downloaded,
                overall_total,
                source: source.to_string(),
            });
        };
        download_file(&client, &bases, model_dir, spec, &mut on_file_progress)?;
        done_bytes += spec.size;
    }
    Ok(())
}

/// 下载单文件：3 次重试，每次遍历全部基址；写 `.tmp` 校验后改名。
/// Downloads one file: 3 retries cycling all bases each time; writes `.tmp` then
/// renames after verification.
fn download_file(
    client: &reqwest::blocking::Client,
    bases: &[String],
    model_dir: &Path,
    spec: &ModelFileSpec,
    on_progress: &mut dyn FnMut(u64, u64, &str),
) -> Result<(), ModelError> {
    let target = model_dir.join(spec.name);
    let tmp = model_dir.join(format!("{}.tmp", spec.name));
    let mut last_err = String::new();

    for attempt in 0..3 {
        for base in bases {
            let url = format!("{base}/{}", spec.name);
            on_progress(0, 0, host_of(base));
            match download_to(client, &url, &tmp, host_of(base), on_progress) {
                Ok(()) => match file_sha256(&tmp) {
                    Ok(hash) if hash == spec.sha256 => {
                        std::fs::rename(&tmp, &target)?;
                        tracing::info!(file = spec.name, "模型文件下载并校验通过");
                        return Ok(());
                    }
                    Ok(_) => {
                        last_err = format!("{url}：SHA-256 不匹配");
                        let _ = std::fs::remove_file(&tmp);
                    }
                    Err(e) => {
                        last_err = format!("{url}：校验读取失败 {e}");
                        let _ = std::fs::remove_file(&tmp);
                    }
                },
                Err(e) => {
                    last_err = format!("{url}：{e}");
                    let _ = std::fs::remove_file(&tmp);
                }
            }
        }
        tracing::warn!(file = spec.name, attempt = attempt + 1, error = %last_err, "下载失败，准备重试");
        std::thread::sleep(std::time::Duration::from_millis(
            2000 * (attempt as u64 + 1),
        ));
    }
    Err(ModelError::Download(last_err))
}

/// 流式下载到 `tmp`，进度回调按文件上报（total 未知时为 0）。
/// Streams the download into `tmp`; per-file progress (total 0 when unknown).
fn download_to(
    client: &reqwest::blocking::Client,
    url: &str,
    tmp: &Path,
    source: &str,
    on_progress: &mut dyn FnMut(u64, u64, &str),
) -> Result<(), String> {
    let mut resp = client
        .get(url)
        .send()
        .and_then(|r| r.error_for_status())
        .map_err(|e| e.to_string())?;
    let total = resp.content_length().unwrap_or(0);
    let mut file = std::fs::File::create(tmp).map_err(|e| e.to_string())?;
    use std::io::{Read, Write};
    let mut buffer = [0u8; 64 * 1024];
    let mut downloaded: u64 = 0;
    let mut last_report: u64 = 0;
    loop {
        let n = resp.read(&mut buffer).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        file.write_all(&buffer[..n]).map_err(|e| e.to_string())?;
        downloaded += n as u64;
        // 进度按 512KB 节流，避免事件风暴
        // throttle progress to every 512 KiB to avoid event storms
        if downloaded - last_report >= 512 * 1024 || downloaded == total {
            last_report = downloaded;
            on_progress(downloaded, total, source);
        }
    }
    file.flush().map_err(|e| e.to_string())?;
    if total > 0 && downloaded != total {
        return Err(format!("下载不完整：{downloaded}/{total} 字节"));
    }
    Ok(())
}

/// 从基址提取 host 作为进度事件里的源标识（镜像 URL 前缀拼接形式同样适用）。
/// Extracts the host from a base URL as the source label (works for the
/// mirror prefix + full-URL form too).
fn host_of(base: &str) -> &str {
    base.split("://")
        .nth(1)
        .unwrap_or(base)
        .split('/')
        .next()
        .unwrap_or(base)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_urls_includes_official() {
        let urls = base_urls();
        assert!(urls.iter().any(|u| u == OFFICIAL_BASE_URL));
        assert_eq!(urls.last().unwrap(), OFFICIAL_BASE_URL, "官方源应兜底");
    }

    #[test]
    fn resolve_builtin_and_path() {
        assert!(resolve_model_dir("latex-ocr").ends_with("models/latex-ocr"));
        assert_eq!(
            resolve_model_dir("/tmp/my-model"),
            PathBuf::from("/tmp/my-model")
        );
    }

    #[test]
    fn model_files_have_unique_names() {
        let mut names: Vec<_> = MODEL_FILES.iter().map(|f| f.name).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), MODEL_FILES.len());
    }

    #[test]
    fn file_ready_detects_hash_mismatch() {
        let dir = tempfile::tempdir().unwrap();
        let spec = &MODEL_FILES[3]; // tokenizer.json
        std::fs::write(dir.path().join(spec.name), b"not the tokenizer").unwrap();
        assert!(!file_ready(dir.path(), spec));
    }

    #[test]
    fn file_sha256_matches_known_vector() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("empty");
        std::fs::write(&path, b"").unwrap();
        // sha256("") 的公认值
        // the well-known sha256 of an empty input
        assert_eq!(
            file_sha256(&path).unwrap(),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    /// 真实网络下载单文件（tokenizer.json，24KB）：验证新用户下载链路
    /// （流式下载 → SHA-256 校验 → tmp 改名）。
    /// Real-network download of one file (tokenizer.json, 24KB): verifies the
    /// fresh-user path (streamed download → SHA-256 check → tmp rename).
    #[test]
    #[ignore = "需要网络：cargo test --lib download -- --ignored --nocapture"]
    fn download_fetches_and_verifies_single_file() {
        let dir = tempfile::tempdir().unwrap();
        let spec = &MODEL_FILES[3]; // tokenizer.json
        let client = reqwest::blocking::Client::new();
        let mut progress_calls = 0;
        download_file(
            &client,
            &base_urls(),
            dir.path(),
            spec,
            &mut |_file, _done, _total| progress_calls += 1,
        )
        .expect("下载失败");
        assert!(
            progress_calls > 0,
            "进度回调应至少触发一次（新用户 UI 依赖它）"
        );
        assert!(file_ready(dir.path(), spec), "下载后文件应校验通过");
        assert!(
            !dir.path().join(format!("{}.tmp", spec.name)).exists(),
            "tmp 文件应已被改名消费"
        );
    }
}
