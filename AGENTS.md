# AGENTS.md

## 交流语言

始终使用中文与用户交流。代码、commit message、PR 描述等技术输出也用中文。

## 写作要求

所有面向人读的文本（注释、文档、ADR、commit message），遵守以下原则：

- 善于总结材料，去粗取精，反映本质；不堆砌细节。
- 真懂才能写好；逻辑清晰；用词准确；观点鲜明。
- 废话尽量除去；不用夸大的修饰词。
- 通俗亲切，先讲已知再讲未知。

## 编码准则

1. **写代码前先读懂**。看 import、看测试、确认 API 真实存在（ort 的 API 以 docs.rs 对应版本为准，不凭记忆）。
2. **动手前想清楚**。说出假设、点明取舍、标出架构决策。
3. **避免过度工程**。不过早抽象、不投机式错误处理、不做没必要的可配置性。
4. **精准改动**。diff 最小化、贴合现有风格、不重新格式化。
5. **验证**。修 bug 先写复现测试；测行为不测实现；按改动范围分层验证。
6. **目标驱动**。模糊任务转成可验证标准；多步任务先出计划。
7. **调试**。读完错误信息、先复现、一次只改一处、不懂根因不加 workaround。
8. **依赖克制**。先看项目已有/标准库能否做到。
9. **沟通**。说做了什么为什么、标出顾虑、commit message 具体。
10. **常见失败模式**。厨房水槽式顺手重构、隐形决策、乐观路径、风格漂移、失控重构。

**注释风格**：所有源码注释中英双语，中文在前、英文在后（与 altgo 一致）。

## 项目概览

**freetex** 是开源桌面公式识别工具（SimpleTex 的本地离线替代）：截图 / 粘贴 / 上传图片，用 **LaTeX-OCR（pix2tex）ONNX 模型**（RapidLaTeXOCR 转换版）在本地识别为 LaTeX，结果自动复制并展示，支持 KaTeX 实时预览、LaTeX / `$…$` / `$$…$$` / MathML(Word) 多格式复制、本地历史记录。技术架构移植自 altgo（Tauri 2 + React 18，核心逻辑不 import Tauri，平台能力与引擎全部藏在 trait seam 后）。

## 构建与测试命令

```bash
# 仅 Rust（无 GUI）
cargo build --manifest-path=src-tauri/Cargo.toml
cargo test  --manifest-path=src-tauri/Cargo.toml
cargo fmt   --manifest-path=src-tauri/Cargo.toml -- --check
cargo clippy --manifest-path=src-tauri/Cargo.tsoml -- -D warnings   # 注意路径拼写

# 前端
cd frontend && npm install && npm run build

# Tauri GUI
cargo tauri dev        # 开发（需先 npm install）
cargo tauri build      # 生产构建

make build / test / fmt / lint / run / clean
```

## 架构

移植自 altgo 的骨架，语音链路替换为公式识别链路：

```
Hotkey Listener ─┐
                 ├→ SnipManager（选区窗）→ capture（截屏裁剪）→┐
上传/粘贴 ───────┴──────────────────────────────────────────→ Recognition Pipeline
                                                              （引擎懒加载 → ort 推理）
                                                                 ↓
                                              Clipboard + History + 前端事件
```

### 模块（src-tauri/src/）

| 模块 | 职责 |
|---|---|
| `lib.rs` | Tauri 装配：managed state、流水线线程、托盘、关窗驻留托盘、16 个命令 |
| `cmd.rs` | Tauri 命令层：只做 IPC 转换（camelCase DTO ↔ snake_case Config） |
| `config.rs` / `config_store.rs` | TOML 配置 + 补丁校验写盘（全字段 serde default） |
| `engine/` | 引擎 seam：`Recognizer` trait；`pix2tex.rs`（ort 推理：resizer 宽度适配循环 → encoder → 自回归 decoder top-k 采样）、`preprocess.rs`（pad/minmax_size/normalize，与 RapidLaTeXOCR Python 逐步对齐）、`tokenizer.rs`（BPE 解码）、`postprocess.rs`（LaTeX 空白清理正则） |
| `model.rs` | 模型下载管理：SHA-256 校验、tmp+rename 原子落盘、3 次重试、`FREETEX_MODEL_BASE_URL` 镜像覆盖、进度回调 |
| `capture.rs` | `screenshots` crate 抓屏 + `image` 裁剪（全局物理像素坐标） |
| `hotkey/` | 快捷键监听（Windows `WH_KEYBOARD_LL`；Linux `evtest`），`keymap.rs` 维护键名→平台码映射 |
| `hotkey_manager.rs` | 快捷键生命周期：按下事件 → 触发截图流程 |
| `pipeline/` | 组合根，**不 import Tauri**：单任务串行主循环、积压合并、`PipelineSink`/`Clipboard`/`Recognizer` 三个 seam |
| `pipeline_controller.rs` | 流水线生命周期（start/stop/restart，save_config 时重建） |
| `tauri_sink.rs` | `PipelineSink` → 前端事件（`pipeline-status` / `recognition-result` / `pipeline-error`） |
| `snip.rs` | 选区窗管理：摆窗、逻辑→物理坐标换算、提交任务 |
| `clipboard.rs` | arboard 剪贴板（text + html；html 承载 MathML 给 Word） |
| `tray.rs` | 托盘：显示主窗 / 截图识别 / 退出 |
| `updater.rs` | 应用自动更新（移植自 altgo ADR-0004）：静默/手动双检查（手动 10 秒超时、分类错误）；NSIS 与 AppImage 就地更新，deb/rpm 外部引导；`UpdateProvider` trait seam；识别进行中拒绝重启 |
| `history.rs` | `history.json`（camelCase JSON，全局 IO 锁，只存文本不存图） |
| `resource.rs` | 路径与线程数工具 |

### 前端（frontend/src/）

双入口：`index.html`（主窗）+ `snip.html`（选区窗）。页面：Home（识别 + KaTeX 预览 + 多格式复制）、History、Settings（模型下载 / 快捷键 / 输出 / 主题 / 语言）。i18n 与主题照搬 altgo 的轻量方案（内联字典 + storage 事件跨窗同步）。公式渲染 KaTeX，MathML 导出 Temml。

### IPC 契约

命令：`get_config` `save_config` `start_snip` `cancel_snip` `confirm_snip` `recognize_image_bytes` `recognize_image_path` `copy_text` `copy_mathml` `list_models` `download_model` `delete_model` `resolve_model` `list_history` `delete_history_entries` `clear_history` `check_update` `install_update` `get_releases_url`。

事件：`pipeline-status` `recognition-result` `pipeline-error` `history-updated` `model-download-progress` `model-download-finished` `hotkey-backend` `snip-shown`。

## 关键技术事实（防止知识幻觉）

- **ort 2.0.0-rc.13**：`Session::run(&mut self)` 需要可变引用且 Session 不可 Clone → 会话包 `Mutex`；`session.inputs()` 是**方法**返回 `&[Outlet]`，名字取 `outlet.name()`；输出提取用 `try_extract_array::<f32>()` 返回 `ndarray::ArrayViewD`；`inputs!` 宏直接返回值不返回 Result。
- **推理流程**（RapidLaTeXOCR master）：预处理 mean=0.7931 std=0.1738；pad 裁剪文字外接框并对齐 32；max 672×192 min 32×32；BOS=1 EOS=2，max_seq_len=512；decoder 每步喂全序列（无 KV cache）取最后一位 logits；top-k=10% 词表 + 温度 1e-5（等价贪心）。
- 模型 SHA-256 与下载地址在 `model.rs` 常量里；模型放 `<config>/freetex/models/latex-ocr/`。

## 发布（1.0.0 起）

打 tag `v*` 推送触发 `release.yml`：校验（validate-release.sh，含 updater endpoint 与仓库一致性检查）→ 双平台测试 → Linux x64/arm64（deb/rpm/AppImage）+ Windows x64/arm64（NSIS/MSI）构建（`TAURI_SIGNING_PRIVATE_KEY` 签名 updater 产物）→ 合并 latest.json（merge-updater-json.sh）→ GitHub Release（含 checksums.txt）。

**发布前必改**：`tauri.conf.json` 里 updater `endpoints` 与 README 徽章的 `cislunarspace/freetex` 换成实际仓库（validate 脚本会拦）。GitHub secrets 需配 `TAURI_SIGNING_PRIVATE_KEY`（`~/.tauri/freetex-updater.key` 内容）与 `TAURI_KEY_PASSWORD`（本密钥为空串）。私钥丢失则无法再发更新。

## 测试说明

- 单元测试在各文件 `#[cfg(test)]`；`cargo test --lib` 不需要 GUI 与模型。
- 引擎端到端测试用 `#[ignore]` 标记：`cargo test --lib e2e -- --ignored`，需要 `.dev/models/` 有模型文件与 `.dev/test-images/`。
- 平台模块（钩子、evtest、截屏）只有构造/冒烟测试。
