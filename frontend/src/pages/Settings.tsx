import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useTranslation } from "../i18n";
import { useTheme, type Theme } from "../ThemeContext";
import {
  IS_MOBILE,
  useModelDownloadProgress,
  useModelDownloadFinished,
  useRefreshSignal,
} from "../hooks/useTauri";

interface ModelFileDto {
  name: string;
  ready: boolean;
}

interface ModelInfoDto {
  name: string;
  dir: string;
  ready: boolean;
  files: ModelFileDto[];
}

interface ConfigDto {
  snip: {
    hotkey: string;
    autoCopy: boolean;
    copyFormat: string;
    hideMainDuringSnip: boolean;
  };
  engine: {
    model: string;
    numThreads: number;
  };
}

/** 与 Rust 侧 `SUPPORTED_HOTKEYS` 一致。 */
const SUPPORTED_HOTKEYS = [
  "F1", "F2", "F3", "F4", "F5", "F6", "F7", "F8", "F9", "F10", "F11", "F12",
  "PrintScreen", "Insert", "Delete", "Home", "End", "PageUp", "PageDown",
  "Alt_R", "Control_R", "Shift_R",
];

export default function Settings() {
  const { t, lang, setLang } = useTranslation();
  const { theme, setTheme } = useTheme();
  const [config, setConfig] = useState<ConfigDto | null>(null);
  const [models, setModels] = useState<ModelInfoDto[]>([]);
  const [backend, setBackend] = useState<string | null>(null);
  const [saved, setSaved] = useState(false);
  const [downloading, setDownloading] = useState(false);
  const progress = useModelDownloadProgress();
  const finished = useModelDownloadFinished();
  const historySignal = useRefreshSignal("history-updated");

  const reload = useCallback(() => {
    invoke<ConfigDto>("get_config")
      .then(setConfig)
      .catch(() => undefined);
    invoke<ModelInfoDto[]>("list_models")
      .then(setModels)
      .catch(() => undefined);
  }, []);

  useEffect(reload, [reload, historySignal]);

  useEffect(() => {
    if (downloading && finished) {
      setDownloading(false);
      reload();
    }
  }, [finished, downloading, reload]);

  useEffect(() => {
    const p = listen<{ backend: string }>("hotkey-backend", (e) =>
      setBackend(e.payload.backend)
    );
    return () => {
      p.then((fn) => fn()).catch(() => undefined);
    };
  }, []);

  const patch = (mutate: (draft: ConfigDto) => void) => {
    setConfig((prev) => {
      if (!prev) return prev;
      const draft = JSON.parse(JSON.stringify(prev)) as ConfigDto;
      mutate(draft);
      return draft;
    });
  };

  const save = () => {
    if (!config) return;
    invoke<ConfigDto>("save_config", { patch: config })
      .then(() => {
        setSaved(true);
        setTimeout(() => setSaved(false), 1500);
      })
      .catch(() => undefined);
  };

  const download = (name: string) => {
    setDownloading(true);
    invoke("download_model", { name }).catch(() => setDownloading(false));
  };

  const remove = (name: string) => {
    invoke("delete_model", { name })
      .then(reload)
      .catch(() => undefined);
  };

  if (!config) {
    return <div className="placeholder">…</div>;
  }

  const model = models[0];

  return (
    <div className="settings-page">
      <section className="panel">
        <h2>{t("settingsModel")}</h2>
        {model && (
          <>
            <div className="model-row">
              <div>
                <strong>{model.name}</strong>
                <span className={`badge ${model.ready ? "ok" : "warn"}`}>
                  {model.ready ? t("modelReady") : t("modelNotReady")}
                </span>
                <div className="muted small">{model.dir}</div>
              </div>
              <div className="model-files">
                {model.files.map((f) => (
                  <span key={f.name} className={`file-chip ${f.ready ? "ok" : ""}`}>
                    {f.name} {f.ready ? "✓" : "…"}
                  </span>
                ))}
              </div>
            </div>
            <div className="btn-row">
              {!model.ready && (
                <button
                  className="btn primary"
                  onClick={() => download(model.name)}
                  disabled={downloading}
                >
                  {downloading ? t("downloading") : `${t("download")}（${t("totalSize")}）`}
                </button>
              )}
              {model.ready && (
                <button className="btn danger ghost" onClick={() => remove(model.name)}>
                  {t("delete")}
                </button>
              )}
            </div>
            {downloading && progress && progress.total > 0 && (
              <div className="progress">
                <div
                  className="progress-bar"
                  style={{
                    width: `${Math.min(100, (progress.downloaded / progress.total) * 100)}%`,
                  }}
                />
                <span className="muted small">
                  {progress.fileName} · {(progress.downloaded / 1048576).toFixed(1)} /{" "}
                  {(progress.total / 1048576).toFixed(1)} MB
                </span>
              </div>
            )}
          </>
        )}
      </section>

      {/* 移动端无全局快捷键，隐藏该面板
          Mobile has no global hotkeys; the panel stays desktop-only */}
      {!IS_MOBILE && (
        <section className="panel">
          <h2>{t("settingsHotkey")}</h2>
          <div className="form-row">
            <select
              value={config.snip.hotkey}
              onChange={(e) => patch((d) => (d.snip.hotkey = e.target.value))}
            >
              {SUPPORTED_HOTKEYS.map((key) => (
                <option key={key} value={key}>
                  {key}
                </option>
              ))}
            </select>
            {backend && (
              <span className="muted small">
                {t("backend")}: {backend}
              </span>
            )}
          </div>
        </section>
      )}

      <section className="panel">
        <h2>{t("settingsOutput")}</h2>
        <div className="form-row">
          <label>
            <input
              type="checkbox"
              checked={config.snip.autoCopy}
              onChange={(e) => patch((d) => (d.snip.autoCopy = e.target.checked))}
            />
            {t("autoCopy")}
          </label>
        </div>
        <div className="form-row">
          <span className="muted">{t("copyFormat")}</span>
          <select
            value={config.snip.copyFormat}
            onChange={(e) => patch((d) => (d.snip.copyFormat = e.target.value))}
          >
            <option value="latex">{t("formatLatex")}</option>
            <option value="inline_math">{t("formatInline")}</option>
            <option value="display_math">{t("formatDisplay")}</option>
          </select>
        </div>
      </section>

      <section className="panel">
        <h2>{t("theme")} / {t("language")}</h2>
        <div className="form-row">
          <select value={theme} onChange={(e) => setTheme(e.target.value as Theme)}>
            <option value="light">{t("themeLight")}</option>
            <option value="dark">{t("themeDark")}</option>
            <option value="system">{t("themeSystem")}</option>
          </select>
          <select value={lang} onChange={(e) => setLang(e.target.value as "zh" | "en")}>
            <option value="zh">中文</option>
            <option value="en">English</option>
          </select>
        </div>
      </section>

      <div className="btn-row">
        <button className="btn primary" onClick={save}>
          {saved ? t("saved") : t("saveSettings")}
        </button>
      </div>

      <section className="panel about">
        <h2>{t("settingsAbout")}</h2>
        <p className="muted">{t("aboutText")}</p>
        <p className="muted small">freetex v1.0.0 · LaTeX-OCR (pix2tex) · MIT</p>

        <UpdateCard />
      </section>
    </div>
  );
}

interface UpdateCheckResult {
  hasUpdate: boolean;
  currentVersion: string;
  latestVersion: string;
  body?: string;
  date?: string;
  supportTier: "in_place" | "external";
}

/** 更新卡片：移动端只留发布页链接；桌面端手动检查 + 就地安装（NSIS/AppImage）或外部引导（deb/rpm）。 */
/** Update card: mobile shows just a releases-page link; desktop does manual check + in-place
 * install (NSIS/AppImage) or external guidance (deb/rpm). */
function UpdateCard() {
  const { t } = useTranslation();
  const [releasesUrl, setReleasesUrl] = useState<string | null>(null);

  useEffect(() => {
    invoke<string>("get_releases_url")
      .then(setReleasesUrl)
      .catch(() => undefined);
  }, []);

  if (IS_MOBILE) {
    return (
      <div className="update-card">
        {releasesUrl && (
          <a className="btn" href={releasesUrl} target="_blank" rel="noreferrer">
            {t("gotoDownload")}
          </a>
        )}
      </div>
    );
  }

  const [checking, setChecking] = useState(false);
  const [installing, setInstalling] = useState(false);
  const [result, setResult] = useState<UpdateCheckResult | null>(null);
  const [error, setError] = useState<string | null>(null);

  const check = async () => {
    setChecking(true);
    setError(null);
    setResult(null);
    try {
      const res = await invoke<UpdateCheckResult>("check_update", { mode: "manual" });
      setResult(res);
    } catch (e) {
      setError(String(e));
    } finally {
      setChecking(false);
    }
  };

  const install = async () => {
    setInstalling(true);
    setError(null);
    try {
      await invoke("install_update");
    } catch (e) {
      setError(String(e));
      setInstalling(false);
    }
  };

  return (
    <div className="update-card">
      <div className="btn-row">
        <button className="btn" onClick={check} disabled={checking || installing}>
          {checking ? t("checkingUpdate") : t("checkUpdate")}
        </button>
        {result?.hasUpdate && result.supportTier === "in_place" && (
          <button className="btn primary" onClick={install} disabled={installing}>
            {installing ? t("installingUpdate") : t("installUpdate")}
          </button>
        )}
        {result?.hasUpdate && result.supportTier === "external" && releasesUrl && (
          <a className="btn" href={releasesUrl} target="_blank" rel="noreferrer">
            {t("gotoDownload")}
          </a>
        )}
      </div>
      {result && !result.hasUpdate && <p className="muted small">{t("upToDate")}</p>}
      {result?.hasUpdate && (
        <div className="update-info">
          <p>
            <strong>
              {t("newVersion")}：v{result.currentVersion} → v{result.latestVersion}
            </strong>
          </p>
          {result.body && <pre className="muted small update-notes">{result.body}</pre>}
          {result.supportTier === "in_place" ? (
            <p className="muted small">{t("updateRestartHint")}</p>
          ) : (
            <p className="muted small">{t("externalUpdateHint")}</p>
          )}
        </div>
      )}
      {error && (
        <p className="muted small">
          {t("updateFailed")}：{error}
        </p>
      )}
    </div>
  );
}
