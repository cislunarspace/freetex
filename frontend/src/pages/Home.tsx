import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Camera, SquareDashed } from "lucide-react";
import { useTranslation } from "../i18n";
import { useStatus, type RecognitionResult } from "../hooks/useTauri";
import { renderLatex, renderMathml } from "../katex";

type CopyKind = "latex" | "inline" | "display" | "mathml";

export default function Home() {
  const { t } = useTranslation();
  const status = useStatus();
  const [imageData, setImageData] = useState<string | null>(null);
  const [result, setResult] = useState<RecognitionResult | null>(null);
  const [latex, setLatex] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [copied, setCopied] = useState<CopyKind | null>(null);
  const [modelReady, setModelReady] = useState<boolean | null>(null);
  const [dragOver, setDragOver] = useState(false);
  const fileInput = useRef<HTMLInputElement>(null);

  useEffect(() => {
    const cleanups = [
      listen<RecognitionResult>("recognition-result", (e) => {
        setResult(e.payload);
        setLatex(e.payload.latex);
        setError(null);
        invoke<{ ready: boolean }>("resolve_model")
          .then((r) => setModelReady(r.ready))
          .catch(() => undefined);
      }),
      listen<{ message: string }>("pipeline-error", (e) =>
        setError(e.payload.message)
      ),
    ];
    return () => cleanups.forEach((p) => p.then((fn) => fn()).catch(() => undefined));
  }, []);

  useEffect(() => {
    invoke<{ ready: boolean }>("resolve_model")
      .then((r) => setModelReady(r.ready))
      .catch(() => setModelReady(null));
  }, [status]);

  const recognizeBytes = useCallback((bytes: Uint8Array) => {
    const reader = new FileReader();
    reader.onload = () => setImageData(reader.result as string);
    reader.readAsDataURL(new Blob([bytes]));
    setError(null);
    invoke("recognize_image_bytes", { bytes: Array.from(bytes) }).catch((e) =>
      setError(String(e))
    );
  }, []);

  const loadFile = useCallback(
    (file: File) => {
      file.arrayBuffer().then((buf) => recognizeBytes(new Uint8Array(buf)));
    },
    [recognizeBytes]
  );

  // Ctrl+V 粘贴图片直接识别
  // Ctrl+V pastes an image straight into recognition
  useEffect(() => {
    const onPaste = (e: ClipboardEvent) => {
      const file = Array.from(e.clipboardData?.files ?? []).find((f) =>
        f.type.startsWith("image/")
      );
      if (file) loadFile(file);
    };
    window.addEventListener("paste", onPaste);
    return () => window.removeEventListener("paste", onPaste);
  }, [loadFile]);

  const startSnip = () => invoke("start_snip").catch((e) => setError(String(e)));

  const copy = async (kind: CopyKind) => {
    if (!latex.trim()) return;
    try {
      if (kind === "mathml") {
        await invoke("copy_mathml", { mathml: renderMathml(latex), plain: latex });
      } else if (kind === "inline") {
        await invoke("copy_text", { text: `$${latex}$` });
      } else if (kind === "display") {
        await invoke("copy_text", { text: `$$${latex}$$` });
      } else {
        await invoke("copy_text", { text: latex });
      }
      setCopied(kind);
      setTimeout(() => setCopied(null), 1500);
    } catch (e) {
      setError(String(e));
    }
  };

  const html = latex.trim() ? renderLatex(latex) : null;

  return (
    <div className="home">
      <div className="home-actions">
        <button className="btn primary" onClick={startSnip}>
          <Camera size={16} />
          {t("snipButton")}
        </button>
        <button
          className="btn"
          onClick={() => fileInput.current?.click()}
          disabled={status === "recognizing"}
        >
          <SquareDashed size={16} />
          {t("clickUpload")}
        </button>
        <input
          ref={fileInput}
          type="file"
          accept="image/png,image/jpeg,image/bmp"
          hidden
          onChange={(e) => {
            const file = e.target.files?.[0];
            if (file) loadFile(file);
            e.target.value = "";
          }}
        />
      </div>

      {modelReady === false && (
        <div className="banner warn">
          {t("modelMissing")} —{" "}
          <a href="#/settings">{t("navSettings")}</a>
        </div>
      )}
      {error && (
        <div className="banner error">
          <strong>{t("errorTitle")}：</strong>
          {error}
        </div>
      )}

      <div
        className={`dropzone ${dragOver ? "drag" : ""}`}
        onDragOver={(e) => {
          e.preventDefault();
          setDragOver(true);
        }}
        onDragLeave={() => setDragOver(false)}
        onDrop={(e) => {
          e.preventDefault();
          setDragOver(false);
          const file = Array.from(e.dataTransfer.files).find((f) =>
            f.type.startsWith("image/")
          );
          if (file) loadFile(file);
        }}
      >
        <div className="home-grid">
          <section className="panel">
            <h2>{t("originalTitle")}</h2>
            {imageData ? (
              <img className="preview" src={imageData} alt="original" />
            ) : (
              <div className="placeholder">
                <p>
                  {t("dropHere")}{" "}
                  <a onClick={() => fileInput.current?.click()}>{t("clickUpload")}</a>
                </p>
                <p className="muted">{t("snipHotkeyHint", { key: "F9" })}</p>
              </div>
            )}
          </section>
          <section className="panel">
            <h2>
              {t("resultTitle")}
              {result && (
                <span className="muted elapse">
                  {t("elapseLabel", { ms: result.elapseMs })}
                </span>
              )}
            </h2>
            <div className="katex-view">
              {status === "recognizing" ? (
                <span className="muted">{t("recognizing")}</span>
              ) : html ? (
                <div dangerouslySetInnerHTML={{ __html: html }} />
              ) : (
                <span className="muted">{t("emptyResult")}</span>
              )}
            </div>
            <label className="muted">{t("editLatex")}</label>
            <textarea
              value={latex}
              onChange={(e) => setLatex(e.target.value)}
              spellCheck={false}
              rows={5}
            />
            <div className="copy-row">
              <button className="btn" onClick={() => copy("latex")} disabled={!latex}>
                {copied === "latex" ? t("copied") : t("copyLatex")}
              </button>
              <button className="btn" onClick={() => copy("inline")} disabled={!latex}>
                {copied === "inline" ? t("copied") : t("copyInline")}
              </button>
              <button className="btn" onClick={() => copy("display")} disabled={!latex}>
                {copied === "display" ? t("copied") : t("copyDisplay")}
              </button>
              <button className="btn" onClick={() => copy("mathml")} disabled={!latex}>
                {copied === "mathml" ? t("copied") : t("copyMathml")}
              </button>
            </div>
            {result?.copyFailed && <div className="muted">{t("copyFailedWarn")}</div>}
          </section>
        </div>
      </div>
    </div>
  );
}
