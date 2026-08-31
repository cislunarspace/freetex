<div align="center">

<img src="app-icon.svg" width="110" alt="freetex" />

# freetex

**开源桌面公式识别 · 全程本地离线**

[![CI](https://github.com/cislunarspace/freetex/actions/workflows/ci.yml/badge.svg)](https://github.com/cislunarspace/freetex/actions/workflows/ci.yml)
[![Release](https://github.com/cislunarspace/freetex/actions/workflows/release.yml/badge.svg)](https://github.com/cislunarspace/freetex/actions/workflows/release.yml)
[![Version](https://img.shields.io/github/v/release/cislunarspace/freetex)](https://github.com/cislunarspace/freetex/releases)
[![Downloads](https://img.shields.io/github/downloads/cislunarspace/freetex/total)](https://github.com/cislunarspace/freetex/releases)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20Linux-blue)](https://github.com/cislunarspace/freetex/releases)

**English** | [简体中文](README.zh-CN.md)

</div>

<img src="assets/screenshot.png" alt="freetex 界面" />

截图或粘贴公式图片，本地识别为 LaTeX 并自动复制——SimpleTex 的开源离线替代。图片不出本机：无账号、无上传、无额度。识别引擎为 [LaTeX-OCR（pix2tex）](https://github.com/lukas-blecher/LaTeX-OCR) 的 ONNX 模型，CPU 推理。

## 安装

从 [Releases](https://github.com/cislunarspace/freetex/releases) 下载对应安装包：

| 平台 | 包 |
|---|---|
| Windows x64 / arm64 | `*-setup.exe` 或 `.msi` |
| Linux x64 / arm64 | `.deb` / `.rpm` / `.AppImage`（Ubuntu 22.04 / 24.04 / 26.04 及同版本发行版） |

安装后首次启动，到 **设置** 页下载识别模型（约 180 MB，仅需一次）。

## 功能

- 📸 快捷键截图框选（默认 F9）、拖拽、Ctrl+V 粘贴、上传图片
- ⚡ 本地离线识别，KaTeX 实时预览，LaTeX 源码可直接编辑
- 📋 多格式复制：LaTeX / `$…$` / `$$…$$` / MathML（Word 粘贴即公式）
- 🗂 本地历史记录（只存文本，不存图片）
- 🔄 自动检查更新（NSIS / AppImage 就地更新）
- 🌙 深浅主题、中英双语、系统托盘常驻

## 从源码构建

```bash
git clone https://github.com/cislunarspace/freetex && cd freetex
cd frontend && npm install && cd ..
npx --prefix frontend tauri dev    # 开发
npx --prefix frontend tauri build  # 构建
```

## 相关文档

- [架构说明](docs/architecture.md)
- [配置模板](configs/freetex.toml)（模型镜像等环境变量见其中注释）
- [版本记录](CHANGELOG.md)
- [贡献指南](AGENTS.md)

<div align="center">

## Star History

[![Star History Chart](https://api.star-history.com/svg?repos=cislunarspace/freetex&type=Date)](https://star-history.com/#cislunarspace/freetex&Date)

## License

[MIT](LICENSE)

</div>
