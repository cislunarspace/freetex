//! TOML 配置定义与补丁逻辑。
//!
//! TOML config definitions and patch logic.
//!
//! 序列化契约：配置文件用 snake_case；IPC 边界（`cmd.rs`）用 camelCase DTO 转换。
//! Serialization contract: the config file is snake_case; IPC boundaries (in `cmd.rs`)
//! convert through camelCase DTOs.
//!
//! 所有字段 `serde(default)`：部分配置文件也能加载。
//! Every field is `serde(default)`: partial config files still load.

use serde::{Deserialize, Serialize};

/// 顶层配置。
/// Top-level config.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Config {
    pub snip: SnipConfig,
    pub engine: EngineConfig,
}

/// 截图识别配置。
/// Snip-recognition config.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SnipConfig {
    /// 触发截图的快捷键（单键，如 "F9"、"PrintScreen"、"Alt_R"）
    /// Hotkey that triggers a snip (single key, e.g. "F9", "PrintScreen", "Alt_R")
    pub hotkey: String,
    /// 识别完成后自动复制结果
    /// Copy the result automatically after recognition
    pub auto_copy: bool,
    /// 复制格式：latex | display_math | inline_math
    /// Copy format: latex | display_math | inline_math
    pub copy_format: String,
    /// 截图时隐藏主窗口
    /// Hide the main window while snipping
    pub hide_main_during_snip: bool,
}

impl Default for SnipConfig {
    fn default() -> Self {
        Self {
            hotkey: "F9".to_string(),
            auto_copy: true,
            copy_format: "latex".to_string(),
            hide_main_during_snip: true,
        }
    }
}

/// 识别引擎配置。
/// Recognition-engine config.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct EngineConfig {
    /// 内置模型名或模型目录
    /// Built-in model name or a model directory
    pub model: String,
    /// 推理线程数，0 = 自动
    /// Inference thread count, 0 = auto
    pub num_threads: usize,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            model: "latex-ocr".to_string(),
            num_threads: 0,
        }
    }
}

/// 复制格式枚举。
/// Copy format enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CopyFormat {
    /// 纯 LaTeX（如 \frac{a}{b}）
    /// Raw LaTeX
    Latex,
    /// 块级公式 $$...$$
    /// Display math $$...$$
    DisplayMath,
    /// 行内公式 $...$
    /// Inline math $...$
    InlineMath,
}

impl CopyFormat {
    /// 解析配置字符串，非法值回退 Latex。
    /// Parses the config string; invalid values fall back to Latex.
    pub fn parse(s: &str) -> Self {
        match s {
            "display_math" => Self::DisplayMath,
            "inline_math" => Self::InlineMath,
            _ => Self::Latex,
        }
    }

    /// 按格式包装 LaTeX 文本。
    /// Wraps LaTeX text according to the format.
    pub fn wrap(&self, latex: &str) -> String {
        match self {
            Self::Latex => latex.to_string(),
            Self::DisplayMath => format!("$${latex}$$"),
            Self::InlineMath => format!("${latex}$"),
        }
    }
}

/// 支持的快捷键列表（供配置校验与设置页下拉）。
/// Supported hotkeys (for config validation and the settings dropdown).
pub const SUPPORTED_HOTKEYS: &[&str] = &[
    "F1",
    "F2",
    "F3",
    "F4",
    "F5",
    "F6",
    "F7",
    "F8",
    "F9",
    "F10",
    "F11",
    "F12",
    "PrintScreen",
    "Insert",
    "Delete",
    "Home",
    "End",
    "PageUp",
    "PageDown",
    "Alt_R",
    "Control_R",
    "Shift_R",
];

impl Config {
    /// 校验配置合法性；返回错误时不落盘。
    /// Validates the config; on error nothing is written.
    pub fn validate(&self) -> Result<(), crate::error::ConfigError> {
        if !SUPPORTED_HOTKEYS.contains(&self.snip.hotkey.as_str()) {
            return Err(crate::error::ConfigError::Validation(format!(
                "不支持的热键 '{}'，可选：{}",
                self.snip.hotkey,
                SUPPORTED_HOTKEYS.join(", ")
            )));
        }
        match self.snip.copy_format.as_str() {
            "latex" | "display_math" | "inline_math" => {}
            other => {
                return Err(crate::error::ConfigError::Validation(format!(
                    "非法复制格式 '{other}'"
                )));
            }
        }
        if self.engine.num_threads > 64 {
            return Err(crate::error::ConfigError::Validation(
                "推理线程数不能超过 64".to_string(),
            ));
        }
        if self.engine.model.trim().is_empty() {
            return Err(crate::error::ConfigError::Validation(
                "模型名不能为空".to_string(),
            ));
        }
        Ok(())
    }
}

/// Duration ↔ 毫秒 serde 助手（与 altgo 一致；当前配置未用到，留给扩展）。
/// Duration ↔ milliseconds serde helpers (same as altgo; unused today, kept
/// for future extension).
pub mod duration_ms {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::Duration;

    pub fn serialize<S: Serializer>(v: &Duration, s: S) -> Result<S::Ok, S::Error> {
        v.as_millis().serialize(s)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Duration, D::Error> {
        Ok(Duration::from_millis(u64::deserialize(d)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_valid() {
        assert!(Config::default().validate().is_ok());
    }

    #[test]
    fn partial_config_loads_with_defaults() {
        let cfg: Config = toml::from_str("[snip]\nhotkey = \"PrintScreen\"").unwrap();
        assert_eq!(cfg.snip.hotkey, "PrintScreen");
        assert!(cfg.snip.auto_copy, "默认 auto_copy 应为 true");
        assert_eq!(cfg.engine.model, "latex-ocr", "默认模型应为 latex-ocr");
    }

    #[test]
    fn validate_rejects_bad_hotkey() {
        let cfg = Config {
            snip: SnipConfig {
                hotkey: "Ctrl+Shift+A".to_string(),
                ..SnipConfig::default()
            },
            ..Config::default()
        };
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn validate_rejects_bad_copy_format() {
        let cfg = Config {
            snip: SnipConfig {
                copy_format: "rtf".to_string(),
                ..SnipConfig::default()
            },
            ..Config::default()
        };
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn copy_format_wraps() {
        assert_eq!(CopyFormat::parse("latex").wrap("x^2"), "x^2");
        assert_eq!(CopyFormat::parse("display_math").wrap("x^2"), "$$x^2$$");
        assert_eq!(CopyFormat::parse("inline_math").wrap("x^2"), "$x^2$");
        assert_eq!(CopyFormat::parse("nonsense"), CopyFormat::Latex);
    }

    #[test]
    fn toml_roundtrip_snake_case() {
        let cfg = Config::default();
        let s = toml::to_string(&cfg).unwrap();
        assert!(s.contains("auto_copy"), "TOML 应为 snake_case");
        let back: Config = toml::from_str(&s).unwrap();
        assert_eq!(cfg, back);
    }

    #[test]
    fn duration_ms_roundtrip() {
        use std::time::Duration;

        #[derive(Serialize, Deserialize)]
        struct Wrapper {
            #[serde(with = "duration_ms")]
            v: Duration,
        }
        let w: Wrapper = toml::from_str("v = 250").unwrap();
        assert_eq!(w.v, Duration::from_millis(250));
    }
}
