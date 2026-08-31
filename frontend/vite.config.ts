import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// 双入口：主窗 index.html + 截图选区窗 snip.html（与 altgo 双入口同构）
// Two entries: main window index.html + snip window snip.html (same shape as altgo)
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: "127.0.0.1",
  },
  build: {
    rollupOptions: {
      input: {
        main: "index.html",
        snip: "snip.html",
      },
    },
  },
});
