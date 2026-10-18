import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

// Fixed port so tauri.conf.json's devUrl always matches.
export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: { port: 1420, strictPort: true },
  build: { target: 'chrome105' } // WebView2 is evergreen Chromium
});
