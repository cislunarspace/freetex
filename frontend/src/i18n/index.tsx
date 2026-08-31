// 轻量 i18n：结构照搬 altgo（内联字典 + storage 事件跨窗口同步）。
// Lightweight i18n: same structure as altgo (inline dicts + storage-event sync).

import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";

export type Lang = "zh" | "en";

const STORAGE_KEY = "freetex-lang";

const translations: Record<Lang, Record<string, string>> = {
  zh: {
    appName: "freetex",
    tagline: "开源公式识别",
    navHome: "识别",
    navHistory: "历史",
    navSettings: "设置",
    dropHere: "拖入图片、Ctrl+V 粘贴，或",
    clickUpload: "点击上传",
    snipButton: "截图识别",
    snipHotkeyHint: "按快捷键截图：{key}",
    recognizing: "正在识别…",
    idle: "等待输入",
    done: "识别完成",
    modelMissing: "模型未就绪，请到设置页下载",
    resultTitle: "识别结果",
    originalTitle: "原图",
    elapseLabel: "耗时 {ms} ms",
    copyLatex: "复制 LaTeX",
    copyInline: "复制 $…$",
    copyDisplay: "复制 $$…$$",
    copyMathml: "复制 MathML（Word）",
    copied: "已复制",
    copyFailedWarn: "已复制（剪贴板直写失败，请手动复制）",
    editLatex: "LaTeX 源码（可编辑，预览实时更新）",
    emptyHistory: "暂无历史记录",
    deleteSelected: "删除所选",
    clearAll: "清空全部",
    settingsModel: "识别模型",
    modelReady: "已就绪",
    modelNotReady: "未下载",
    download: "下载",
    delete: "删除",
    downloading: "下载中…",
    settingsHotkey: "截图快捷键",
    settingsOutput: "输出",
    autoCopy: "识别后自动复制",
    copyFormat: "复制格式",
    formatLatex: "纯 LaTeX",
    formatInline: "行内公式 $…$",
    formatDisplay: "块级公式 $$…$$",
    saveSettings: "保存",
    saved: "已保存",
    settingsAbout: "关于",
    aboutText:
      "freetex 是一个开源桌面公式识别工具：截图或粘贴图片，本地离线识别为 LaTeX。识别引擎为 LaTeX-OCR（pix2tex）ONNX 模型。",
    backend: "快捷键后端",
    totalSize: "约 180 MB",
    errorTitle: "出错了",
    supportedKeys: "支持的按键",
    language: "语言",
    theme: "主题",
    themeLight: "浅色",
    themeDark: "深色",
    themeSystem: "跟随系统",
    reRecognize: "重新识别此图",
    emptyResult: "识别结果会显示在这里",
    hideMainHint: "截图时主窗口将自动隐藏",
    localOnly: "全程本地离线，图片不会上传",
    checkUpdate: "检查更新",
    checkingUpdate: "检查中…",
    upToDate: "已是最新版本",
    newVersion: "发现新版本",
    installUpdate: "立即更新",
    installingUpdate: "下载并安装中…",
    updateRestartHint: "安装完成后应用会自动重启",
    externalUpdateHint: "当前安装方式不支持就地更新，请到发布页下载新版本安装包",
    gotoDownload: "前往下载页",
    updateFailed: "检查更新失败",
    autoCheckUpdate: "启动时自动检查更新",
  },
  en: {
    appName: "freetex",
    tagline: "open-source formula OCR",
    navHome: "Recognize",
    navHistory: "History",
    navSettings: "Settings",
    dropHere: "Drop an image, press Ctrl+V to paste, or",
    clickUpload: "browse",
    snipButton: "Snip & Recognize",
    snipHotkeyHint: "Press the hotkey to snip: {key}",
    recognizing: "Recognizing…",
    idle: "Waiting for input",
    done: "Done",
    modelMissing: "Model not ready — download it on the Settings page",
    resultTitle: "Result",
    originalTitle: "Original",
    elapseLabel: "{ms} ms",
    copyLatex: "Copy LaTeX",
    copyInline: "Copy $…$",
    copyDisplay: "Copy $$…$$",
    copyMathml: "Copy MathML (Word)",
    copied: "Copied",
    copyFailedWarn: "Copied (direct clipboard write failed, copy manually)",
    editLatex: "LaTeX source (editable, live preview)",
    emptyHistory: "No history yet",
    deleteSelected: "Delete selected",
    clearAll: "Clear all",
    settingsModel: "Recognition model",
    modelReady: "Ready",
    modelNotReady: "Not downloaded",
    download: "Download",
    delete: "Delete",
    downloading: "Downloading…",
    settingsHotkey: "Snip hotkey",
    settingsOutput: "Output",
    autoCopy: "Copy result automatically",
    copyFormat: "Copy format",
    formatLatex: "Raw LaTeX",
    formatInline: "Inline math $…$",
    formatDisplay: "Display math $$…$$",
    saveSettings: "Save",
    saved: "Saved",
    settingsAbout: "About",
    aboutText:
      "freetex is an open-source desktop formula OCR tool: snip or paste an image and recognize it offline as LaTeX. The engine is a LaTeX-OCR (pix2tex) ONNX model.",
    backend: "Hotkey backend",
    totalSize: "approx. 180 MB",
    errorTitle: "Error",
    supportedKeys: "Supported keys",
    language: "Language",
    theme: "Theme",
    themeLight: "Light",
    themeDark: "Dark",
    themeSystem: "System",
    reRecognize: "Re-recognize this image",
    emptyResult: "Recognized results will appear here",
    hideMainHint: "The main window hides while snipping",
    localOnly: "Runs fully offline — images never leave your machine",
    checkUpdate: "Check for updates",
    checkingUpdate: "Checking…",
    upToDate: "You're up to date",
    newVersion: "New version available",
    installUpdate: "Update now",
    installingUpdate: "Downloading & installing…",
    updateRestartHint: "The app restarts automatically after installing",
    externalUpdateHint: "This install method doesn't support in-place updates — grab the new package from the releases page",
    gotoDownload: "Open releases page",
    updateFailed: "Update check failed",
    autoCheckUpdate: "Check for updates at startup",
  },
};

interface I18nContextValue {
  lang: Lang;
  setLang: (lang: Lang) => void;
  t: (key: string, vars?: Record<string, string | number>) => string;
}

const I18nContext = createContext<I18nContextValue | null>(null);

function initialLang(): Lang {
  const stored = localStorage.getItem(STORAGE_KEY);
  return stored === "en" || stored === "zh" ? stored : "zh";
}

export function TranslationProvider({ children }: { children: ReactNode }) {
  const [lang, setLangState] = useState<Lang>(initialLang);

  // 跨窗口同步：storage 事件（主窗 ↔ snip 窗）+ 本窗 CustomEvent
  // cross-window sync: storage event (main ↔ snip) + local CustomEvent
  useEffect(() => {
    const onStorage = (e: StorageEvent) => {
      if (e.key === STORAGE_KEY && (e.newValue === "en" || e.newValue === "zh")) {
        setLangState(e.newValue);
      }
    };
    const onCustom = () => setLangState(initialLang());
    window.addEventListener("storage", onStorage);
    window.addEventListener("freetex-lang-changed", onCustom);
    return () => {
      window.removeEventListener("storage", onStorage);
      window.removeEventListener("freetex-lang-changed", onCustom);
    };
  }, []);

  const setLang = useCallback((next: Lang) => {
    setLangState(next);
    localStorage.setItem(STORAGE_KEY, next);
    window.dispatchEvent(new CustomEvent("freetex-lang-changed"));
  }, []);

  const t = useCallback(
    (key: string, vars?: Record<string, string | number>) => {
      let text = translations[lang]?.[key] ?? translations.zh[key] ?? key;
      if (vars) {
        for (const [name, value] of Object.entries(vars)) {
          text = text.replace(`{${name}}`, String(value));
        }
      }
      return text;
    },
    [lang]
  );

  const value = useMemo(() => ({ lang, setLang, t }), [lang, setLang, t]);
  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>;
}

export function useTranslation(): I18nContextValue {
  const ctx = useContext(I18nContext);
  if (!ctx) throw new Error("useTranslation must be used inside TranslationProvider");
  return ctx;
}
