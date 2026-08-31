# freetex

![freetex](app-icon.svg)

[![CI](https://github.com/your-name/freetex/actions/workflows/ci.yml/badge.svg)](https://github.com/your-name/freetex/actions/workflows/ci.yml)
[![Release](https://github.com/your-name/freetex/actions/workflows/release.yml/badge.svg)](https://github.com/your-name/freetex/actions/workflows/release.yml)
[![Version](https://img.shields.io/github/v/release/your-name/freetex)](https://github.com/your-name/freetex/releases)
[![Downloads](https://img.shields.io/github/downloads/your-name/freetex/total)](https://github.com/your-name/freetex/releases)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20Linux-blue)](https://github.com/your-name/freetex/releases)
[![Engine](https://img.shields.io/badge/engine-LaTeX--OCR%20(pix2tex)-orange)](docs/architecture.md)

**English** | [简体中文](README.zh-CN.md)

**freetex** is an open-source desktop formula OCR tool — a local, offline alternative to SimpleTex: snip or paste an image, recognize it locally as LaTeX, get the result on your clipboard in multiple formats.

- Images never leave your machine: fully offline recognition, no accounts, no uploads, no quotas
- Engine: the LaTeX-OCR (pix2tex) ONNX model, running on CPU via onnxruntime (ort)
- Technical architecture ported from [altgo](https://github.com/cislunarspace/altgo) (Tauri 2 + React 18)

## Features

- Global hotkey (F9 by default) → snip a region → recognized automatically → result copied
- Drag & drop / Ctrl+V paste / click to upload images in the main window
- Live KaTeX preview with an editable LaTeX source
- Multiple copy formats: LaTeX / inline `$…$` / display `$$…$$` / MathML (pastes as an equation in Word)
- Local history: view, copy, delete, clear (text only, images are never stored)
- Model download and management on the Settings page: SHA-256 verified, progress display, mirror override
- Auto-update: silent check at startup, manual check & one-click install on the Settings page (in-place for NSIS / AppImage, guided download for deb / rpm)
- Tray icon, light/dark themes, English & Chinese UI

## Installation

Grab the package for your platform from [Releases](https://github.com/your-name/freetex/releases):

| Platform | Package |
|---|---|
| Windows x64 / arm64 | `*-setup.exe` (NSIS) or `.msi` |
| Linux x64 / arm64 | `.deb`, `.rpm`, or `.AppImage` |

On first launch, download the recognition model (~180 MB, one time) on the **Settings** page; set `FREETEX_MODEL_BASE_URL` to a mirror when GitHub is slow.

## Platform support

- Windows 10+ (x86_64 & arm64)
- Linux, Ubuntu 22.04+ (x86_64 & aarch64); the hotkey needs `evtest` and membership of the `input` group

## Quick start (from source)

```bash
git clone <repo> && cd freetex
make build       # = npm install + cargo tauri build
```

Or run in dev mode:

```bash
cd frontend && npm install && cd ..
cargo tauri dev
```

Download the recognition model on the **Settings** page (about 180 MB, one time), then press F9 to snip.

## Commands

```bash
make test    # cargo test --lib (57 unit/integration tests)
make fmt
make lint
make run     # cargo tauri dev
make clean
```

Engine end-to-end test (requires local models in `.dev/models/`):

```bash
cargo test --manifest-path=src-tauri/Cargo.toml --lib e2e -- --ignored --nocapture
```

## Configuration

The config file lives at `~/.config/freetex/freetex.toml` (`%APPDATA%/freetex/freetex.toml` on Windows):

```toml
[snip]
hotkey = "F9"              # snip hotkey (single key)
auto_copy = true           # copy the result automatically
copy_format = "latex"      # latex | display_math | inline_math
hide_main_during_snip = true

[engine]
model = "latex-ocr"        # built-in model name or a model directory
num_threads = 0            # 0 = auto
```

Models are downloaded from the RapidAI/RapidLaTeXOCR GitHub Releases by default; set `FREETEX_MODEL_BASE_URL` to a mirror directory when GitHub is slow (the directory must contain the four files with the same names).

## Documentation

- [`docs/architecture.md`](docs/architecture.md): system architecture
- [`AGENTS.md`](AGENTS.md): engineering conventions and key API facts
- [`CHANGELOG.md`](CHANGELOG.md): version history

## Star History

[![Star History Chart](https://api.star-history.com/svg?repos=your-name/freetex&type=Date)](https://star-history.com/#your-name/freetex&Date)

## License

[MIT License](LICENSE)
