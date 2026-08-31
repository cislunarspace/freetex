# 架构

freetex 的技术架构移植自 [altgo](https://github.com/cislunarspace/altgo)：Tauri 2 + React 18 桌面应用，业务核心不依赖 Tauri，平台能力与引擎全部藏在 trait seam 后面。把 altgo 的「录音 → 转写 → 润色 → 输出」语音链路替换为「截图 → 识别 → 输出」公式识别链路。

## 一句话设计

**流水线跑在专用 OS 线程上，通过四个 seam 与外界交互**：

```
Hotkey Listener ─┐
                 ├→ SnipManager（选区窗）→ capture（截屏裁剪）→┐
上传/粘贴 ───────┴──────────────────────────────────────────→ Recognition Pipeline
                                                              （引擎懒加载 → ort 推理）
                                                                 ↓
                                              Clipboard + History + 前端事件
```

| seam | 定义处 | 生产实现 |
|---|---|---|
| `Recognizer` | `engine/mod.rs` | `pix2tex.rs`（ort 跑三个 ONNX） |
| `PipelineSink` | `pipeline/sink.rs` | `tauri_sink.rs`（事件 → 前端） |
| `Clipboard` | `clipboard.rs` | arboard（text + html） |
| `HotkeyListener` | `hotkey/mod.rs` | Windows 钩子 / Linux evtest |

`pipeline/` 组合根完全不 import Tauri，因此 `cargo test --lib` 不需要 GUI。

## 模块地图（src-tauri/src/）

- **`lib.rs`** —— 装配：managed state（`Arc<ConfigStore>`、`Arc<HistoryStore>`、`PipelineController`、`HotkeyManager`、`SnipManager`），托盘，关窗驻留，16 个命令注册。
- **`cmd.rs`** —— 命令层：只做 IPC 转换（camelCase DTO ↔ snake_case Config），`restart_pipeline` 编排。
- **`pipeline/`** —— 单任务串行主循环：收任务 → 引擎懒加载 → 识别 → 剪贴板 + 历史 → 结果事件。积压任务自动合并（只处理最新一张）。对应 altgo ADR-0003 的单次识别互斥。
- **`engine/pix2tex.rs`** —— LaTeX-OCR（pix2tex）ONNX 推理，与 RapidLaTeXOCR 的 Python 实现逐步对齐：
  1. 预处理（`preprocess.rs`）：亮度/反转 alpha 选择 → min-max 归一化 → 文字外接框裁剪 → 32 对齐白底填充 → 超 672×192 缩小、不足 32×32 补齐 → `(v − 0.7931·255) / (0.1738·255)`；
  2. resizer 循环：`image_resizer.onnx` 预测宽度档位（(argmax+1)×32），按比例迭代收敛（≤10 轮）；
  3. encoder：图像张量 → 上下文；
  4. decoder：自回归（无 KV cache，每步喂全序列取末位 logits），top-k=10% 词表 + 温度 1e-5 采样（饱和为贪心），BOS=1 / EOS=2 / 上限 512；
  5. tokenizer（`tokenizer.rs`）：BPE 词表解码，跳过特殊 token，`Ġ` 还原为空格；
  6. postprocess（`postprocess.rs`）：fancy-regex 折叠多余空白，保留 `\ ` 转义空格。
- **`model.rs`** —— 模型管理：唯一内置模型 `latex-ocr`（encoder 89MB + decoder 51MB + resizer 39MB + tokenizer 24KB），SHA-256 校验、tmp+rename 原子落盘、3 次重试、`FREETEX_MODEL_BASE_URL` 镜像覆盖、进度回调。
- **`capture.rs`** —— `screenshots` crate 抓屏 + `image` 裁剪；注意 screenshots 0.8 依赖 image 0.24，经原始 RGBA 缓冲桥接到我们的 image 0.25。
- **`hotkey/` + `hotkey_manager.rs`** —— 快捷键：Windows `WH_KEYBOARD_LL` 钩子（移植自 altgo，过滤注入事件）；Linux `evtest` 读 `/dev/input/event*`。按下事件经转发线程触发截图流程；换键即重建监听器。
- **`snip.rs`** —— 选区窗管理：把 `snip.html` 窗口摆到主显示器，前端拖拽选区，Rust 把逻辑像素乘以 scale_factor 换算成全局物理像素后提交任务；识别前等待 150ms 让窗口从合成器消失。
- **`tray.rs`** —— 托盘菜单：显示主窗 / 截图识别 / 退出。
- **`updater.rs`** —— 应用自动更新（移植 altgo ADR-0004）：`UpdateProvider` trait seam 包住 tauri-plugin-updater；`check_update_core` 带 10 秒超时与错误分类（超时/网络/签名/限流）；`detect_support_tier` 按运行方式分级——Windows NSIS 与 AppImage 就地更新（下载→校验→重启），deb/rpm 引导到发布页（`get_releases_url` 从 updater endpoint 推导）。安装前经 `PipelineController::current_status` 检查，识别进行中拒绝。启动时前端 Layout 静默检查（失败静默），设置页手动检查 + 安装。
- **`history.rs`** —— `history.json`（camelCase），只存 LaTeX 文本不存图片；全局 IO 锁。
- **`config.rs` / `config_store.rs`** —— TOML 配置，全字段 serde default，补丁校验后写盘；校验失败不落盘。`save_config` 重启流水线（引擎懒加载，重启代价低），快捷键变化时单独重建监听器。

## 前端（frontend/src/）

双入口 `index.html`（主窗）+ `snip.html`（选区窗）。Home 页承担 SimpleTex 的核心交互：图片预览 + KaTeX 预览 + 可编辑 LaTeX + 四种复制（LaTeX / `$…$` / `$$…$$` / MathML）。MathML 经 Temml 渲染，以 `text/html` 剪贴板格式写入，Word 粘贴即公式。i18n 与主题照搬 altgo 的轻量方案（内联字典 + storage 事件跨窗口同步）。

## IPC 契约

16 个命令：`get_config` `save_config` `start_snip` `cancel_snip` `confirm_snip` `recognize_image_bytes` `recognize_image_path` `copy_text` `copy_mathml` `list_models` `download_model` `delete_model` `resolve_model` `list_history` `delete_history_entries` `clear_history`。

8 个事件：`pipeline-status` `recognition-result` `pipeline-error` `history-updated` `model-download-progress` `model-download-finished` `hotkey-backend` `snip-shown`。IPC 一律 camelCase。

## 与 altgo 的差异（有意为之）

- 无悬浮窗（识别结果展示在主窗；MVP 取舍，roadmap 可加）。
- 无按键状态机（freetex 只响应按下，不需要长按/双击）。
- 无 LLM 润色（后续里程碑）；自动更新已移植（updater.rs，同款分级策略）。
- 识别线程用 std mpsc + std thread 而非 tokio runtime（没有录音超时/并发需求，同步更诚实）。
- Linux 剪贴板用 arboard 而非 xclip 子进程（单一实现，seam 保留）。
- 发布矩阵为 Linux x64/arm64 + Windows x64/arm64（同 altgo）；无 AUR/winget/scoop 分发渠道（后续按需）。

## MVP 边界（roadmap）

- 截图只覆盖主显示器；多显示器、跨屏拖拽待支持。
- 不支持组合键热键（SimpleTex 的 Ctrl+Shift+A 形态）；仅单键。
- 无 PDF 识别、AI 编辑、翻译（SimpleTex 的增值功能，需要 LLM 接入后规划）。
- ort 预编译库未覆盖的平台（如 Windows arm64）需系统 onnxruntime（`ort` 的 `load-dynamic`）。
