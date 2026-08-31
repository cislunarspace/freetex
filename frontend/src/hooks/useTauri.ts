// Tauri 集成 hooks：事件订阅与状态转发（结构照搬 altgo 的 useTauri.ts）。
// Tauri integration hooks: event subscription and state forwarding (like altgo's).

import { useEffect, useState } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export function useTauriEvent<T>(event: string, initial: T): T {
  const [state, setState] = useState<T>(initial);
  useEffect(() => {
    let unlisten: UnlistenFn | undefined;
    let disposed = false;
    listen<T>(event, (e) => setState(e.payload))
      .then((fn) => {
        if (disposed) fn();
        else unlisten = fn;
      })
      .catch(() => undefined);
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [event]);
  return state;
}

/** pipeline-status 事件的 status 字段。 */
export type PipelineStatus = "idle" | "recognizing" | "done" | "stopped";

export interface RecognitionResult {
  latex: string;
  elapseMs: number;
  source: "snip" | "image";
  copyFailed: boolean;
}

export interface ModelDownloadProgress {
  fileName: string;
  downloaded: number;
  total: number;
}

export interface ModelDownloadFinished {
  success: boolean;
  message?: string;
}

export function useStatus(): PipelineStatus {
  return useTauriEvent<PipelineStatus>(
    "pipeline-status",
    "idle"
  );
}

export function useModelDownloadProgress(): ModelDownloadProgress | null {
  const [progress, setProgress] = useState<ModelDownloadProgress | null>(null);
  useEffect(() => {
    let unlisten: UnlistenFn | undefined;
    let disposed = false;
    listen<ModelDownloadProgress>("model-download-progress", (e) =>
      setProgress(e.payload)
    )
      .then((fn) => {
        if (disposed) fn();
        else unlisten = fn;
      })
      .catch(() => undefined);
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);
  return progress;
}

export function useModelDownloadFinished(): ModelDownloadFinished | null {
  const [finished, setFinished] = useState<ModelDownloadFinished | null>(null);
  useEffect(() => {
    let unlisten: UnlistenFn | undefined;
    let disposed = false;
    listen<ModelDownloadFinished>("model-download-finished", (e) =>
      setFinished(e.payload)
    )
      .then((fn) => {
        if (disposed) fn();
        else unlisten = fn;
      })
      .catch(() => undefined);
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);
  return finished;
}

/** 递增计数器事件：用于触发刷新（如 history-updated）。 */
export function useRefreshSignal(event: string): number {
  const [count, setCount] = useState(0);
  useEffect(() => {
    let unlisten: UnlistenFn | undefined;
    let disposed = false;
    listen(event, () => setCount((c) => c + 1))
      .then((fn) => {
        if (disposed) fn();
        else unlisten = fn;
      })
      .catch(() => undefined);
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [event]);
  return count;
}
