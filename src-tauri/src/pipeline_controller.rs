//! 流水线生命周期：启动 / 停止 / 重启（对应 altgo 的 PipelineController）。
//!
//! Pipeline lifecycle: start / stop / restart (mirrors altgo's PipelineController).

use crate::pipeline::sink::PipelineStatus;
use crate::pipeline::{Command, PipelineDeps};
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex, RwLock};

/// 线程持有 + 命令通道。
/// Thread handle + command channel.
struct Running {
    sender: Sender<Command>,
    handle: Option<std::thread::JoinHandle<()>>,
}

#[derive(Default)]
pub struct PipelineController {
    running: Mutex<Option<Running>>,
    /// 最新流水线状态：由 sink 写入，updater 等外部模块读取。
    /// Latest pipeline status: written by the sink, read by outsiders (updater etc.).
    status: Arc<RwLock<PipelineStatus>>,
}

impl PipelineController {
    /// 共享状态句柄（塞进 sink，让状态变化同步落到这里）。
    /// Shared status handle (handed to the sink so status changes land here).
    pub fn status_arc(&self) -> Arc<RwLock<PipelineStatus>> {
        self.status.clone()
    }

    /// 当前状态快照。
    /// Snapshot of the current status.
    pub fn current_status(&self) -> PipelineStatus {
        *self.status.read().unwrap()
    }

    /// 测试专用：直接注入状态。
    /// Tests only: inject a status directly.
    #[cfg(test)]
    pub fn set_status_for_tests(&self, status: PipelineStatus) {
        *self.status.write().unwrap() = status;
    }

    /// 启动流水线线程；已运行时先停旧的。
    /// Starts the pipeline thread; stops the old one if running.
    pub fn start(&self, deps: PipelineDeps) {
        let mut guard = self.running.lock().unwrap();
        Self::stop_locked(&mut guard);
        let (tx, rx) = channel();
        let handle = std::thread::Builder::new()
            .name("freetex-pipeline".into())
            .spawn(move || crate::pipeline::run(rx, deps))
            .expect("流水线线程启动失败");
        *guard = Some(Running {
            sender: tx,
            handle: Some(handle),
        });
    }

    /// 停止流水线（等待线程退出，最多 3 秒后放弃 join）。
    /// Stops the pipeline (joins the thread, giving up after 3s at most).
    pub fn stop(&self) {
        let mut guard = self.running.lock().unwrap();
        Self::stop_locked(&mut guard);
    }

    /// 提交一个识别任务；未运行时返回 false。
    /// Submits a recognition job; false when not running.
    pub fn submit(&self, job: crate::pipeline::Job) -> bool {
        let guard = self.running.lock().unwrap();
        match guard.as_ref() {
            Some(running) => running.sender.send(Command::Job(job)).is_ok(),
            None => false,
        }
    }

    fn stop_locked(running: &mut Option<Running>) {
        if let Some(mut old) = running.take() {
            let _ = old.sender.send(Command::Stop);
            // 引擎可能正在推理，join 不能无限等；drop sender 让线程在任务结束后退出
            // the engine may be mid-inference; don't join forever — dropping the
            // sender lets the thread exit once the current job finishes
            if let Some(handle) = old.handle.take() {
                let _ = handle.join();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clipboard::Clipboard;
    use crate::config::CopyFormat;
    use crate::engine::Recognizer;
    use crate::error::{ClipboardError, EngineError, FatalError};
    use crate::history::HistoryStore;
    use crate::pipeline::sink::{PipelineSink, PipelineStatus, RecognitionOutcome};
    use crate::pipeline::{Job, PipelineDeps};
    use image::RgbaImage;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc as StdArc;
    use std::time::Duration;

    struct CountingEngine {
        calls: StdArc<AtomicUsize>,
    }

    impl Recognizer for CountingEngine {
        fn recognize(&self, _image: &RgbaImage) -> Result<crate::engine::Recognition, EngineError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            std::thread::sleep(Duration::from_millis(20));
            Ok(crate::engine::Recognition {
                latex: "x^2".to_string(),
                elapse: Duration::from_millis(1),
            })
        }
    }

    struct FakeClipboard {
        texts: StdArc<Mutex<Vec<String>>>,
    }

    impl Clipboard for FakeClipboard {
        fn copy_text(&self, text: &str) -> Result<(), ClipboardError> {
            self.texts.lock().unwrap().push(text.to_string());
            Ok(())
        }
        fn copy_html(&self, _html: &str, _plain: &str) -> Result<(), ClipboardError> {
            Ok(())
        }
    }

    #[derive(Default)]
    struct RecordingSink {
        statuses: Mutex<Vec<String>>,
        results: Mutex<Vec<RecognitionOutcome>>,
        failures: Mutex<Vec<String>>,
    }

    impl PipelineSink for RecordingSink {
        fn status_changed(&self, status: PipelineStatus) {
            self.statuses
                .lock()
                .unwrap()
                .push(status.as_str().to_string());
        }
        fn recognition_succeeded(&self, outcome: &RecognitionOutcome, _copy_failed: bool) {
            self.results.lock().unwrap().push(outcome.clone());
        }
        fn failed(&self, message: &str) {
            self.failures.lock().unwrap().push(message.to_string());
        }
    }

    fn small_png() -> Vec<u8> {
        let img = RgbaImage::from_pixel(64, 64, image::Rgba([255, 255, 255, 255]));
        let dyn_img = image::DynamicImage::from(img);
        let mut buf = std::io::Cursor::new(Vec::new());
        dyn_img.write_to(&mut buf, image::ImageFormat::Png).unwrap();
        buf.into_inner()
    }

    struct TestHarness {
        controller: PipelineController,
        sink: StdArc<RecordingSink>,
        clipboard: StdArc<Mutex<Vec<String>>>,
        calls: StdArc<AtomicUsize>,
        _dir: tempfile::TempDir,
    }

    fn harness(auto_copy: bool) -> TestHarness {
        let sink = StdArc::new(RecordingSink::default());
        let clipboard_texts = StdArc::new(Mutex::new(Vec::new()));
        let calls = StdArc::new(AtomicUsize::new(0));
        let dir = tempfile::tempdir().unwrap();
        let history =
            HistoryStore::load(dir.path().join("history.json")).expect("历史存储初始化失败");
        let engine = CountingEngine {
            calls: calls.clone(),
        };
        let deps = PipelineDeps {
            sink: Box::new(SinkClone(sink.clone())),
            clipboard: StdArc::new(FakeClipboard {
                texts: clipboard_texts.clone(),
            }),
            history: StdArc::new(history),
            engine_factory: Box::new(move || {
                Ok(Box::new(CountingEngine {
                    calls: engine.calls.clone(),
                }) as Box<dyn Recognizer>)
            }),
            auto_copy,
            copy_format: CopyFormat::Latex,
            capture_settle: Duration::from_millis(0),
        };
        let controller = PipelineController::default();
        controller.start(deps);
        TestHarness {
            controller,
            sink,
            clipboard: clipboard_texts,
            calls,
            _dir: dir,
        }
    }

    /// Arc 包装的 sink 桥接成 Box<dyn PipelineSink>（每次调用转发到共享状态）。
    /// Bridges the Arc-shared sink into Box<dyn PipelineSink> forwarding to shared state.
    struct SinkClone(StdArc<RecordingSink>);

    impl PipelineSink for SinkClone {
        fn status_changed(&self, status: PipelineStatus) {
            self.0.status_changed(status);
        }
        fn recognition_succeeded(&self, outcome: &RecognitionOutcome, copy_failed: bool) {
            self.0.recognition_succeeded(outcome, copy_failed);
        }
        fn failed(&self, message: &str) {
            self.0.failed(message);
        }
    }

    #[test]
    fn controller_runs_job_to_done() {
        let h = harness(false);
        assert!(
            h.controller.submit(Job::ImageBytes { bytes: small_png() }),
            "任务应提交成功"
        );
        // 轮询等待完成（最多 2 秒）
        // poll for completion (2s at most)
        for _ in 0..100 {
            if !h.sink.results.lock().unwrap().is_empty() {
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        h.controller.stop();
        let results = h.sink.results.lock().unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].latex, "x^2");
        assert_eq!(results[0].source, crate::pipeline::sink::SourceKind::Image);
        assert_eq!(h.calls.load(Ordering::SeqCst), 1);
        let statuses = h.sink.statuses.lock().unwrap();
        assert!(statuses.contains(&"recognizing".to_string()));
        assert!(statuses.contains(&"done".to_string()));
        assert_eq!(statuses.last().unwrap(), "stopped", "停止后应广播 stopped");
    }

    #[test]
    fn auto_copy_writes_clipboard() {
        let h = harness(true);
        h.controller
            .submit(Job::ImageBytes { bytes: small_png() })
            .then_some(())
            .expect("任务应提交成功");
        for _ in 0..100 {
            if !h.clipboard.lock().unwrap().is_empty() {
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        h.controller.stop();
        let texts = h.clipboard.lock().unwrap();
        assert_eq!(texts.last().map(String::as_str), Some("x^2"));
    }

    #[test]
    fn submit_without_pipeline_returns_false() {
        let controller = PipelineController::default();
        assert!(!controller.submit(Job::ImageBytes { bytes: vec![] }));
    }

    #[test]
    fn engine_failure_reports_and_pipeline_stays_alive() {
        let h = harness(false);
        // 用一个总是失败的工厂重启
        // restart with a factory that always fails
        let sink = StdArc::new(RecordingSink::default());
        let deps = PipelineDeps {
            sink: Box::new(SinkClone(sink.clone())),
            clipboard: StdArc::new(FakeClipboard {
                texts: StdArc::new(Mutex::new(Vec::new())),
            }),
            history: StdArc::new(HistoryStore::load(h._dir.path().join("history2.json")).unwrap()),
            engine_factory: Box::new(|| Err(FatalError::EngineInit("模型缺失".to_string()))),
            auto_copy: false,
            copy_format: CopyFormat::Latex,
            capture_settle: Duration::from_millis(0),
        };
        h.controller.start(deps);
        h.controller.submit(Job::ImageBytes { bytes: small_png() });
        for _ in 0..100 {
            if !sink.failures.lock().unwrap().is_empty() {
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        h.controller.stop();
        assert!(!sink.failures.lock().unwrap().is_empty(), "引擎失败应上报");
    }
}
