import { defineConfig } from 'vitest/config'
import react from '@vitejs/plugin-react'

const tauriHost = process.env.TAURI_DEV_HOST

// https://vite.dev/config/
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    host: tauriHost || '127.0.0.1',
    port: 5173,
    strictPort: true,
    hmr: {
      protocol: 'ws',
      host: tauriHost || '127.0.0.1',
      port: 1421,
    },
    watch: {
      ignored: ['**/src-tauri/**'],
    },
  },
  build: {
    sourcemap: Boolean(process.env.TAURI_DEBUG),
  },
  test: {
    environment: 'jsdom',
    globals: true,
    setupFiles: './src/test/setup.ts',
  },
})
