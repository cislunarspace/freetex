//! 识别流水线：组合根，完全不 import Tauri。
//!
//! The recognition pipeline: composition root, never imports Tauri.
//!
//! 与 altgo 的 voice_pipeline 同一设计：框架交互收进 `PipelineSink` seam，
//! 平台能力收进 `Clipboard` seam，引擎收进 `Recognizer` seam；主循环在
//! 专用 OS 线程上串行处理任务（单次识别互斥，对应 altgo ADR-0003）。
//! Same design as altgo's voice_pipeline: framework interaction behind the
//! `PipelineSink` seam, platform ability behind `Clipboard`, engine behind
//! `Recognizer`; the main loop processes jobs serially on a dedicated OS thread
//! (single-recognition mutex, mirroring altgo ADR-0003).

pub mod sink;

use crate::clipboard::Clipboard;
use crate::config::CopyFormat;
use crate::engine::Recognizer;
use crate::error::{FatalError, RecoverableError};
use crate::history::HistoryStore;
use sink::{PipelineSink, RecognitionOutcome, SourceKind};
use std::sync::mpsc::{Receiver, TryRecvError};
use std::sync::Arc;
use std::time::Duration;

/// 一次识别任务的来源。
/// Origin of one recognition job.
#[derive(Debug, Clone)]
pub enum Job {
    /// 截图选区（全局物理像素）
    /// snip selection (global physical pixels)
    SnipRect { x: i32, y: i32, w: u32, h: u32 },
    /// 直接图片字节（粘贴 / 上传）
    /// raw image bytes (paste / upload)
    ImageBytes { bytes: Vec<u8> },
}

/// 主循环命令。
/// Main-loop commands.
#[derive(Debug)]
pub enum Command {
    Job(Job),
    Stop,
}

/// 引擎工厂：按当前配置构建识别器（线程内懒调用）。
/// Engine factory: builds a recognizer from current config (lazy, in-thread).
pub type EngineFactory = dyn Fn() -> Result<Box<dyn Recognizer>, FatalError> + Send;

/// 流水线依赖（构造时一次性决定，对应 altgo 的 builder 模式）。
/// Pipeline dependencies (decided once at construction, like altgo's builder).
pub struct PipelineDeps {
    pub sink: Box<dyn PipelineSink>,
    pub clipboard: Arc<dyn Clipboard>,
    pub history: Arc<HistoryStore>,
    pub engine_factory: Box<EngineFactory>,
    pub auto_copy: bool,
    pub copy_format: CopyFormat,
    /// 截图后等待窗口隐藏的缓冲时间
    /// grace period after snipping for the window to hide
    pub capture_settle: Duration,
}

/// 主循环。`Stop` 或通道关闭后线程退出，最后广播 `Stopped`。
/// The main loop. Exits on `Stop` or channel close, broadcasting `Stopped` last.
pub fn run(rx: Receiver<Command>, deps: PipelineDeps) {
    let mut engine: Option<Box<dyn Recognizer>> = None;

    deps.sink.status_changed(sink::PipelineStatus::Idle);
    while let Ok(command) = rx.recv() {
        match command {
            Command::Stop => break,
            Command::Job(job) => {
                // 合并积压任务：只处理最新一个，避免连环旧截图
                // coalesce backlog: keep only the newest job
                let job = coalesce(&rx, job);
                if engine.is_none() {
                    match (deps.engine_factory)() {
                        Ok(e) => engine = Some(e),
                        Err(fatal) => {
                            deps.sink.failed(&fatal.to_string());
                            continue;
                        }
                    }
                }
                process(&deps, engine.as_deref().expect("engine loaded"), job);
            }
        }
    }
    engine.take();
    deps.sink.status_changed(sink::PipelineStatus::Stopped);
}

/// 吸干通道里积压的任务，返回最新的一个。
/// Drains queued jobs and returns the newest.
fn coalesce(rx: &Receiver<Command>, first: Job) -> Job {
    let mut latest = first;
    loop {
        match rx.try_recv() {
            Ok(Command::Job(job)) => latest = job,
            Ok(Command::Stop) | Err(TryRecvError::Disconnected) => break,
            Err(TryRecvError::Empty) => break,
        }
    }
    latest
}

/// 执行一次识别：状态 → 识别 → 剪贴板 + 历史 → 结果事件。
/// Runs one recognition: status → recognize → clipboard + history → result event.
fn process(deps: &PipelineDeps, engine: &dyn Recognizer, job: Job) {
    deps.sink.status_changed(sink::PipelineStatus::Recognizing);

    let result = match &job {
        Job::SnipRect { x, y, w, h } => {
            // 等选区窗从合成器中消失，避免把选区 UI 拍进截图
            // wait for the snip window to vanish from the compositor first
            std::thread::sleep(deps.capture_settle);
            crate::capture::capture_rect(*x, *y, *w, *h)
                .map_err(|e| RecoverableError::Capture(e.to_string()))
                .and_then(|img| {
                    engine
                        .recognize(&img)
                        .map_err(|e| RecoverableError::Recognition(e.to_string()))
                })
        }
        Job::ImageBytes { bytes } => crate::engine::preprocess::load_rgba(bytes)
            .map_err(|e| RecoverableError::Recognition(e.to_string()))
            .and_then(|img| {
                engine
                    .recognize(&img)
                    .map_err(|e| RecoverableError::Recognition(e.to_string()))
            }),
    };

    let outcome = match result {
        Ok(recognition) => {
            let source = match job {
                Job::SnipRect { .. } => SourceKind::Snip,
                Job::ImageBytes { .. } => SourceKind::Image,
            };
            RecognitionOutcome {
                latex: recognition.latex.clone(),
                elapse_ms: recognition.elapse.as_millis() as u64,
                source,
            }
        }
        Err(err) => {
            deps.sink.status_changed(sink::PipelineStatus::Idle);
            deps.sink.failed(&err.to_string());
            return;
        }
    };

    // 次要失败（剪贴板 / 历史）只记日志，不吞掉结果
    // secondary failures (clipboard / history) only log, never swallow the result
    let mut copy_failed = false;
    if deps.auto_copy && !outcome.latex.is_empty() {
        let text = deps.copy_format.wrap(&outcome.latex);
        if let Err(err) = deps.clipboard.copy_text(&text) {
            tracing::warn!(error = %err, "剪贴板写入失败");
            copy_failed = true;
        }
    }
    if let Err(err) = deps.history.append(&outcome.latex) {
        tracing::warn!(error = %err, "历史写入失败");
    }

    deps.sink.status_changed(sink::PipelineStatus::Done);
    deps.sink.recognition_succeeded(&outcome, copy_failed);
}
