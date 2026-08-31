import { useEffect, useState } from "react";
import { NavLink } from "react-router-dom";
import { invoke } from "@tauri-apps/api/core";
import { Home, History, Settings as SettingsIcon } from "lucide-react";
import { useTranslation } from "../i18n";
import { IS_MOBILE } from "../hooks/useTauri";
import type { ReactNode } from "react";

export default function Layout({ children }: { children: ReactNode }) {
  const { t } = useTranslation();
  const [hasNewUpdate, setHasNewUpdate] = useState(false);

  useEffect(() => {
    // 移动端无应用内更新（更新走发布页），跳过静默检查
    // Mobile has no in-app update (updates come from the releases page); skip the silent check
    if (IS_MOBILE) return;
    // 启动时静默检查更新（网络失败静默忽略，不打扰用户）
    // Silent update check at startup; network failures are ignored silently
    const silentCheck = async () => {
      try {
        const res = await invoke<{ hasUpdate: boolean }>("check_update", {
          mode: "silent",
        });
        if (res?.hasUpdate) setHasNewUpdate(true);
      } catch {
        // 静默检查失败不打扰用户
        // a silent-check failure never disturbs the user
      }
    };
    silentCheck();
  }, []);

  return (
    <div className="app-shell">
      <aside className="sidebar">
        <div className="brand">
          <span className="brand-mark">∑</span>
          <span className="brand-name">{t("appName")}</span>
        </div>
        <nav>
          <NavLink to="/" end className={({ isActive }) => (isActive ? "nav-item active" : "nav-item")}>
            <Home size={18} />
            <span>{t("navHome")}</span>
          </NavLink>
          <NavLink to="/history" className={({ isActive }) => (isActive ? "nav-item active" : "nav-item")}>
            <History size={18} />
            <span>{t("navHistory")}</span>
          </NavLink>
          <NavLink to="/settings" className={({ isActive }) => (isActive ? "nav-item active" : "nav-item")}>
            <SettingsIcon size={18} />
            <span>{t("navSettings")}</span>
            {hasNewUpdate && <span className="update-dot" title={t("newVersion")} />}
          </NavLink>
        </nav>
        <div className="sidebar-footer">{t("localOnly")}</div>
      </aside>
      <main className="content">{children}</main>
    </div>
  );
}
