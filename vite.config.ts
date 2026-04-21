import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

const host = process.env.TAURI_DEV_HOST;

export default defineConfig(async () => ({
  plugins: [react()],
  clearScreen: false,
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
      ignored: ["**/src-tauri/**"],
    },
  },
  build: {
    target: ["es2020", "chrome83", "edge83", "firefox78", "safari14"],
    minify: "esbuild",
    cssMinify: true,
    rollupOptions: {
      output: {
        manualChunks: (id) => {
          if (id.includes('node_modules')) {
            if (id.includes('react/') || id.includes('react-dom/') || id.includes('react-router-dom')) {
              return 'vendor';
            }
            if (id.includes('antd/') || id.includes('@ant-design/')) {
              return 'antd';
            }
            if (id.includes('i18next') || id.includes('react-i18next')) {
              return 'i18n';
            }
            if (id.includes('recharts')) {
              return 'charts';
            }
            if (id.includes('@tauri-apps')) {
              return 'tauri';
            }
          }
        },
      },
    },
  },
  optimizeDeps: {
    entries: ["index.html"],
    include: ["react", "react-dom", "antd", "@ant-design/icons", "react-router-dom"],
    exclude: ["@tauri-apps/api", "@tauri-apps/plugin-dialog", "@tauri-apps/plugin-fs", "@tauri-apps/plugin-opener"],
    esbuildOptions: {
      target: "es2020",
    },
  },
  esbuild: {
    target: "es2020",
  },
}));
