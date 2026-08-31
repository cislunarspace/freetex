<div align="center">

<img src="app-icon.svg" width="110" alt="freetex" />

# freetex

**Open-source desktop formula OCR · fully offline**

[![CI](https://github.com/cislunarspace/freetex/actions/workflows/ci.yml/badge.svg)](https://github.com/cislunarspace/freetex/actions/workflows/ci.yml)
[![Release](https://github.com/cislunarspace/freetex/actions/workflows/release.yml/badge.svg)](https://github.com/cislunarspace/freetex/actions/workflows/release.yml)
[![Version](https://img.shields.io/github/v/release/cislunarspace/freetex)](https://github.com/cislunarspace/freetex/releases)
[![Downloads](https://img.shields.io/github/downloads/cislunarspace/freetex/total)](https://github.com/cislunarspace/freetex/releases)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20Linux-blue)](https://github.com/cislunarspace/freetex/releases)

**English** | [简体中文](README.zh-CN.md)

</div>

<img src="assets/screenshot.png" alt="freetex interface" />

Snip or paste a formula image and freetex recognizes it locally as LaTeX, straight onto your clipboard — an open-source, offline alternative to SimpleTex. Images never leave your machine: no accounts, no uploads, no quotas. The engine is the [LaTeX-OCR (pix2tex)](https://github.com/lukas-blecher/LaTeX-OCR) ONNX model running on CPU.

## Installation

Grab the package for your platform from [Releases](https://github.com/cislunarspace/freetex/releases):

| Platform | Package |
|---|---|
| Windows x64 / arm64 | `*-setup.exe` or `.msi` |
| Linux x64 / arm64 | `.deb` / `.rpm` / `.AppImage` (Ubuntu 22.04 / 24.04 / 26.04 and comparable distros) |

On first launch, download the recognition model (~180 MB, one time) on the **Settings** page.

## Features

- 📸 Snip via hotkey (F9 by default), drag & drop, Ctrl+V paste, or upload
- ⚡ Local offline recognition with live KaTeX preview and an editable LaTeX source
- 📋 Copy as LaTeX / `$…$` / `$$…$$` / MathML (pastes as an equation in Word)
- 🗂 Local history (text only, images are never stored)
- 🔄 Auto-update checks (in-place for NSIS / AppImage)
- 🌙 Light/dark themes, English & Chinese, tray icon

## Build from source

```bash
git clone https://github.com/cislunarspace/freetex && cd freetex
cd frontend && npm install && cd ..
npx --prefix frontend tauri dev    # develop
npx --prefix frontend tauri build  # build
```

## Documentation

- [Architecture](docs/architecture.md)
- [Config template](configs/freetex.toml) (mirror env vars documented inline)
- [Changelog](CHANGELOG.md)
- [Contributing](AGENTS.md)

<div align="center">

## Star History

[![Star History Chart](https://api.star-history.com/svg?repos=cislunarspace/freetex&type=Date)](https://star-history.com/#cislunarspace/freetex&Date)

## License

[MIT](LICENSE)

</div>
