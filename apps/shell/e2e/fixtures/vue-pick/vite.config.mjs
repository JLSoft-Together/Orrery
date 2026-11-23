import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import os from 'node:os'
import path from 'node:path'

export default defineConfig({
  plugins: [vue()],
  cacheDir: path.join(os.tmpdir(), 'ccm-vue-pick-cache'),
  server: { host: '127.0.0.1', strictPort: true },
})
