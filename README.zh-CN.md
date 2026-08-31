# freetex

![freetex](app-icon.svg)

[![CI](https://github.com/your-name/freetex/actions/workflows/ci.yml/badge.svg)](https://github.com/your-name/freetex/actions/workflows/ci.yml)
[![Release](https://github.com/your-name/freetex/actions/workflows/release.yml/badge.svg)](https://github.com/your-name/freetex/actions/workflows/release.yml)
[![Version](https://img.shields.io/github/v/release/your-name/freetex)](https://github.com/your-name/freetex/releases)
[![Downloads](https://img.shields.io/github/downloads/your-name/freetex/total)](https://github.com/your-name/freetex/releases)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20Linux-blue)](https://github.com/your-name/freetex/releases)
[![Engine](https://img.shields.io/badge/engine-LaTeX--OCR%20(pix2tex)-orange)](docs/architecture.md)

[English](README.md) | **简体中文**

**freetex** 是一款开源桌面公式识别工具，SimpleTex 的本地离线替代：截图或粘贴图片，在本地识别为 LaTeX，结果自动复制并可导出多种格式。

- 图片不出本机：识别全程本地离线，无账号、无上传、无额度
- 识别引擎：LaTeX-OCR（pix2tex）ONNX 模型，经 onnxruntime（ort）CPU 推理
- 技术架构移植自 [altgo](https://github.com/cislunarspace/altgo)（Tauri 2 + React 18）

## 功能

- 全局快捷键（默认 F9）截图框选 → 自动识别 → 结果写入剪贴板
- 主窗拖入 / Ctrl+V 粘贴 / 点击上传图片识别
- KaTeX 实时预览，LaTeX 源码可直接编辑
- 多格式复制：LaTeX / 行内 `$…$` / 块级 `$$…$$` / MathML（Word 粘贴即公式）
- 本地历史记录：查看、复制、删除、清空（只存文本，不存图片）
- 模型在设置页下载管理：SHA-256 校验、进度显示、镜像地址可覆盖
- 自动检查更新：启动时静默检查、设置页手动检查与一键更新（NSIS / AppImage 就地更新，deb / rpm 引导下载）
- 托盘常驻、深浅主题、中英双语

## 安装

从 [Releases](https://github.com/your-name/freetex/releases) 下载对应安装包：

| 平台 | 包 |
|---|---|
| Windows x64 / arm64 | `*-setup.exe`（NSIS）或 `.msi` |
| Linux x64 / arm64 | `.deb`、`.rpm` 或 `.AppImage` |

安装后首次启动，到 **设置** 页点击下载识别模型（约 180 MB，仅需一次；国内网络可设 `FREETEX_MODEL_BASE_URL` 指向镜像）。

## 平台支持

- Windows 10+（x86_64 与 arm64）
- Linux（Ubuntu 22.04+，x86_64 与 aarch64；快捷键依赖 `evtest` 与 `input` 组）

## 快速开始（源码构建）

```bash
git clone <repo> && cd freetex
make build       # = npm install + cargo tauri build
```

或开发模式：

```bash
cd frontend && npm install && cd ..
cargo tauri dev
```

首次使用在 **设置** 页下载识别模型（约 180 MB，一次下载），之后按 F9 截图即可。

## 命令

```bash
make test    # cargo test --lib（含 57 个单元/集成测试）
make fmt     # 格式检查
make lint    # clippy
make run     # cargo tauri dev
make clean
```

引擎端到端测试（需要 `.dev/models/` 本地模型）：

```bash
cargo test --manifest-path=src-tauri/Cargo.toml --lib e2e -- --ignored --nocapture
```

## 配置

配置文件在 `~/.config/freetex/freetex.toml`（Windows 为 `%APPDATA%/freetex/freetex.toml`）：

```toml
[snip]
hotkey = "F9"              # 触发截图的快捷键（单键）
auto_copy = true           # 识别后自动复制
copy_format = "latex"      # latex | display_math | inline_math
hide_main_during_snip = true

[engine]
model = "latex-ocr"        # 内置模型名或模型目录
num_threads = 0            # 0 = 自动
```

模型下载地址默认为 RapidAI/RapidLaTeXOCR 的 GitHub Releases；国内网络可设置环境变量 `FREETEX_MODEL_BASE_URL` 指向镜像目录（目录下需有同名四个文件）。

## 文档

- [`docs/architecture.md`](docs/architecture.md)：系统架构
- [`AGENTS.md`](AGENTS.md)：工程约定与关键 API 事实
- [`CHANGELOG.md`](CHANGELOG.md)：版本记录

## Star History

[![Star History Chart](https://api.star-history.com/svg?repos=your-name/freetex&type=Date)](https://star-history.com/#your-name/freetex&Date)

## 许可证

[MIT License](LICENSE)
