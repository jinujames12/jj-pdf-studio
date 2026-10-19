import { defineConfig } from 'vitest/config';
import { svelte } from '@sveltejs/vite-plugin-svelte';

// Fixed port so tauri.conf.json's devUrl always matches.
export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  // Shown in About.
  define: { __BUILD_DATE__: JSON.stringify(new Date().toISOString().slice(0, 10)) },
  server: { port: 1420, strictPort: true },
  build: { target: 'chrome105' }, // WebView2 is evergreen Chromium
  test: { include: ['src/**/*.test.ts'] }
});
