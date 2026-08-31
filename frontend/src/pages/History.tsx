import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Trash2, Copy } from "lucide-react";
import { useTranslation } from "../i18n";
import { useRefreshSignal } from "../hooks/useTauri";
import { renderLatex } from "../katex";

interface HistoryEntry {
  id: string;
  createdAtMs: number;
  latex: string;
}

export default function HistoryPage() {
  const { t } = useTranslation();
  const [entries, setEntries] = useState<HistoryEntry[]>([]);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const signal = useRefreshSignal("history-updated");

  const reload = useCallback(() => {
    invoke<HistoryEntry[]>("list_history")
      .then(setEntries)
      .catch(() => undefined);
  }, []);

  useEffect(reload, [reload, signal]);

  const toggle = (id: string) => {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const copyOne = (latex: string) => {
    invoke("copy_text", { text: latex }).catch(() => undefined);
  };

  const deleteSelected = () => {
    invoke("delete_history_entries", { ids: Array.from(selected) })
      .then(() => setSelected(new Set()))
      .catch(() => undefined);
  };

  const clearAll = () => {
    invoke("clear_history")
      .then(() => setSelected(new Set()))
      .catch(() => undefined);
  };

  return (
    <div className="history-page">
      <div className="history-actions">
        <button className="btn danger" onClick={deleteSelected} disabled={selected.size === 0}>
          <Trash2 size={15} />
          {t("deleteSelected")} ({selected.size})
        </button>
        <button className="btn danger ghost" onClick={clearAll} disabled={entries.length === 0}>
          {t("clearAll")}
        </button>
      </div>
      {entries.length === 0 ? (
        <div className="placeholder">{t("emptyHistory")}</div>
      ) : (
        <ul className="history-list">
          {entries.map((entry) => (
            <li key={entry.id} className={selected.has(entry.id) ? "selected" : ""}>
              <label className="history-check">
                <input
                  type="checkbox"
                  checked={selected.has(entry.id)}
                  onChange={() => toggle(entry.id)}
                />
              </label>
              <div
                className="history-latex"
                dangerouslySetInnerHTML={{ __html: renderLatex(entry.latex) }}
              />
              <div className="history-meta">
                <span className="muted">
                  {new Date(entry.createdAtMs).toLocaleString()}
                </span>
                <button className="btn ghost" onClick={() => copyOne(entry.latex)}>
                  <Copy size={14} />
                </button>
              </div>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
