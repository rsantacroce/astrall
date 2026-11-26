import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

// TAURI_DEV_HOST é definido pelo `tauri ios dev` para o aparelho alcançar o servidor.
const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: {
    port: 5183,
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: "ws", host, port: 5184 } : undefined,
    watch: { ignored: ["**/src-tauri/**", "**/target/**"] },
  },
  build: { target: "es2022" },
});
