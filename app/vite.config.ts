import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(async () => ({
  plugins: [react()],

  // Tek ~1.5MB chunk yerine vendor ailelerine böl: paralel-yüklenebilir + app
  // kodu değişince vendor cache korunur. Sıra önemli (özel olanlar önce).
  build: {
    rollupOptions: {
      output: {
        manualChunks(id: string) {
          if (!id.includes("node_modules")) return undefined;
          if (id.includes("react-force-graph") || id.includes("d3-force") || id.includes("/d3-"))
            return "graph";
          if (id.includes("@codemirror") || id.includes("@uiw") || id.includes("@lezer"))
            return "editor";
          if (id.includes("@xterm")) return "term";
          if (
            id.includes("react-markdown") ||
            id.includes("remark") ||
            id.includes("micromark") ||
            id.includes("mdast") ||
            id.includes("unist") ||
            id.includes("hast")
          )
            return "markdown";
          // Tam paket adı: `includes("react")` preact/react-kapsule'u da yakalıyordu.
          if (/[\\/]node_modules[\\/](react|react-dom|scheduler)[\\/]/.test(id)) return "react";
          // Catch-all "vendor" YOK: force-graph/prop-types/vfile gibi geçişli bağımlılıkları
          // ayrı chunk'a koymak vendor<->graph/markdown/react döngüsü kuruyordu; prod build'de
          // "Cannot access 'h' before initialization" → boş pencere. Atanmamış modülleri
          // Rollup, onları kullanan chunk'a yerleştirir (chunk grafiği döngüsüz).
          return undefined;
        },
      },
    },
  },

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },
}));
