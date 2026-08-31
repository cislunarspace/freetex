// 截图选区窗：拖拽画框，松开即识别；Esc 取消。
// Snip window: drag a rectangle, release to recognize; Esc cancels.
import { useCallback, useEffect, useRef, useState } from "react";
import ReactDOM from "react-dom/client";
import { invoke } from "@tauri-apps/api/core";
import "./styles/snip.css";

export default function SnipApp() {
  const [rect, setRect] = useState<{ x: number; y: number; w: number; h: number } | null>(null);
  const dragStart = useRef<{ x: number; y: number } | null>(null);

  const cancel = useCallback(() => {
    invoke("cancel_snip").catch(() => undefined);
  }, []);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") cancel();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [cancel]);

  const onMouseDown = (e: React.MouseEvent) => {
    if (e.button !== 0) return;
    dragStart.current = { x: e.clientX, y: e.clientY };
    setRect({ x: e.clientX, y: e.clientY, w: 0, h: 0 });
  };

  const onMouseMove = (e: React.MouseEvent) => {
    const start = dragStart.current;
    if (!start) return;
    setRect({
      x: Math.min(start.x, e.clientX),
      y: Math.min(start.y, e.clientY),
      w: Math.abs(e.clientX - start.x),
      h: Math.abs(e.clientY - start.y),
    });
  };

  const onMouseUp = () => {
    const start = dragStart.current;
    dragStart.current = null;
    const current = rect;
    if (!start || !current) return;
    // 有效选区（>8px）才提交，否则视为误触取消
    // only submit a real selection (>8px), treat tiny drags as cancels
    if (current.w > 8 && current.h > 8) {
      invoke("confirm_snip", { rect: current }).catch(() => undefined);
    } else {
      setRect(null);
      cancel();
    }
  };

  return (
    <div
      className="snip-root"
      onMouseDown={onMouseDown}
      onMouseMove={onMouseMove}
      onMouseUp={onMouseUp}
    >
      <div className="snip-hint">拖拽框选公式区域，Esc 取消</div>
      {rect && (
        <div
          className="snip-rect"
          style={{ left: rect.x, top: rect.y, width: rect.w, height: rect.h }}
        >
          <div className="snip-size">
            {rect.w} × {rect.h}
          </div>
        </div>
      )}
    </div>
  );
}

ReactDOM.createRoot(document.getElementById("root")!).render(<SnipApp />);
