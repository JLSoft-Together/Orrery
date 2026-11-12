import vue from '@vitejs/plugin-vue'
import { defineConfig } from 'vitest/config'
import { devLogPlugin } from './vite/devLog'
import { serviceWorkerPlugin } from './vite/serviceWorker'

export default defineConfig(({ command }) => ({
  base: '/app/',
  plugins: [vue(), devLogPlugin(), serviceWorkerPlugin()],
  build: {
    outDir: 'dist',
    emptyOutDir: true,
    assetsInlineLimit: 0,
    sourcemap: false,
    target: 'safari16',
    rollupOptions: {
      output: {
        entryFileNames: 'assets/[name]-[hash].js',
        chunkFileNames: 'assets/[name]-[hash].js',
        assetFileNames: 'assets/[name]-[hash][extname]',
      },
    },
  },
  server: { host: true, port: 5180, strictPort: true },
  define: { __DEV_TOOLS__: command === 'serve', __VUE_PROD_DEVTOOLS__: false, __INTLIFY_PROD_DEVTOOLS__: false },
  test: {
    environment: 'happy-dom',
    include: ['src/**/*.test.ts'],
    globals: false,
  },
}))
