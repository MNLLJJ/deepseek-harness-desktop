import { defineConfig } from "vite";

// Tauri 的约定端口：devUrl 在 tauri.conf.json 中指向这里
export default defineConfig({
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
  },
  build: {
    target: "es2022",
    minify: false,
    sourcemap: false,
  },
});
