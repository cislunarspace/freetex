# Changelog

本文件记录 freetex 的显著变更。格式基于 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/)，版本遵循语义化版本。

## v1.0.1 - 2026-09-01

### 新增

- **Android 支持**：相册选图 → 本地离线识别 → 复制，与桌面共用识别链路；移动端布局（底部导航、单列），无托盘/热键/选区截图/应用内更新。
- Android 发布矩阵：`release.yml` 构建签名 APK（arm64-v8a，onnxruntime 静态链接）挂 Release 页。

### 变更

- reqwest 切 rustls（Android 交叉编译不再依赖 openssl，桌面行为不变）。
- gen/android 工程入库（含签名配置），签名走 `TAURI_ANDROID_KEYSTORE_*` 环境变量。

## v1.0.0 - 2026-08-31

首个正式版本：SimpleTex 核心场景的本地离线替代。

### 新增

- 全局快捷键截图框选 → 本地公式识别 → 结果自动复制（默认 F9）。
- 主窗拖入 / Ctrl+V 粘贴 / 上传图片识别。
- LaTeX-OCR（pix2tex）ONNX 引擎：ort CPU 推理，预处理/解码与 RapidLaTeXOCR 逐步对齐，官方测试图全部识别正确。
- 结果多格式复制：LaTeX / `$…$` / `$$…$$` / MathML（Word 粘贴即公式）。
- KaTeX 实时预览与可编辑源码。
- 本地历史记录（只存文本，不存图片）。
- 模型下载管理：SHA-256 校验、进度事件、`FREETEX_MODEL_BASE_URL` 镜像覆盖，新用户首次启动在设置页一键下载。
- 应用自动更新：启动时静默检查 + 设置页手动检查；Windows NSIS 与 Linux AppImage 就地更新，deb/rpm 引导到发布页（minisign 签名校验）。
- 设置页：模型 / 快捷键 / 输出格式 / 主题 / 语言；系统托盘常驻；中英双语。
- 四平台发布矩阵：Linux x64/arm64（deb、rpm、AppImage）+ Windows x64/arm64（NSIS、MSI），Release 流程自动生成 checksums 与 updater latest.json。
- 60+ Rust 单元/集成测试 + 引擎端到端测试（`make e2e`）。
