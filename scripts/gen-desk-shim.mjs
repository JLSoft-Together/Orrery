import { readFileSync, writeFileSync } from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { build } from 'esbuild'

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const PRELOAD = path.join(ROOT, 'apps', 'desktop', 'src', 'preload.ts')
const OUT = path.join(ROOT, 'apps', 'shell', 'src', 'shim.js')

const ELECTRON = `
const tauri = () => window.__TAURI_INTERNALS__
const call = (channel, args) => tauri().invoke('ccm', { channel, args })
const listeners = new Map()
const BINARY = new Set(['ccm:capture-rect'])
const bytes = (v) => (typeof v === 'string' ? Uint8Array.from(atob(v), (c) => c.charCodeAt(0)) : v)
export const ipcRenderer = {
  invoke: (channel, ...args) => (BINARY.has(channel) ? call(channel, args).then(bytes) : call(channel, args)),
  send: (channel, ...args) => {
    call(channel, args).catch(() => undefined)
  },
  sendSync: (channel) => (Object.hasOwn(__ccmShell.sync, channel) ? __ccmShell.sync[channel] : null),
  on: (channel, listener) => {
    const handler = tauri().transformCallback((e) => listener({}, e.payload))
    const id = tauri().invoke('plugin:event|listen', { event: channel, target: { kind: 'Any' }, handler })
    listeners.set(listener, { channel, id })
  },
  removeListener: (channel, listener) => {
    const entry = listeners.get(listener)
    if (!entry || entry.channel !== channel) return
    listeners.delete(listener)
    entry.id.then((eventId) => tauri().invoke('plugin:event|unlisten', { event: channel, eventId })).catch(() => undefined)
  },
}
export const contextBridge = {
  exposeInMainWorld: (name, api) => Object.defineProperty(window, name, { value: Object.freeze(api), writable: false, configurable: false }),
}
const pathWaits = new Map()
let pathListening = false
export const webUtils = {
  getPathForFile: () => '',
  getPathsForFiles: (files) => {
    const wv = window.chrome?.webview
    const none = files.map(() => '')
    if (!files.length || typeof wv?.postMessageWithAdditionalObjects !== 'function') return Promise.resolve(none)
    if (!pathListening) {
      pathListening = true
      wv.addEventListener('message', (e) => {
        const d = e.data
        if (!d || d.ccm !== 'file-paths' || !pathWaits.has(d.id)) return
        const done = pathWaits.get(d.id)
        pathWaits.delete(d.id)
        done(Array.isArray(d.paths) ? d.paths : [])
      })
    }
    const id = crypto.randomUUID()
    return new Promise((resolve) => {
      const timer = setTimeout(() => {
        pathWaits.delete(id)
        resolve(none)
      }, 3000)
      pathWaits.set(id, (paths) => {
        clearTimeout(timer)
        resolve(files.map((_, i) => (typeof paths[i] === 'string' ? paths[i] : '')))
      })
      try {
        wv.postMessageWithAdditionalObjects(JSON.stringify({ ccm: 'file-paths', id }), files)
      } catch {
        clearTimeout(timer)
        pathWaits.delete(id)
        resolve(none)
      }
    })
  },
}
`

const electronShim = {
  name: 'electron-shim',
  setup(b) {
    b.onResolve({ filter: /^electron$/ }, () => ({ path: 'electron', namespace: 'shim' }))
    b.onLoad({ filter: /.*/, namespace: 'shim' }, () => ({ contents: ELECTRON, loader: 'js' }))
  },
}

const result = await build({
  entryPoints: [PRELOAD],
  bundle: true,
  write: false,
  format: 'iife',
  platform: 'browser',
  target: 'es2022',
  legalComments: 'none',
  minifyWhitespace: true,
  charset: 'utf8',
  define: { 'process.platform': '__ccmShell.platform' },
  plugins: [electronShim],
})
const code = result.outputFiles[0].text
if (/\/\*|^\s*\/\//m.test(code)) throw new Error('comment left in the generated shim')

if (process.argv.includes('--check')) {
  let current = ''
  try {
    current = readFileSync(OUT, 'utf8')
  } catch {}
  if (current !== code) {
    console.error(`${path.relative(ROOT, OUT)} is stale: run node scripts/gen-desk-shim.mjs`)
    process.exit(1)
  }
  console.log('shim up to date')
} else {
  writeFileSync(OUT, code)
  console.log(`wrote ${path.relative(ROOT, OUT)} (${code.length} bytes)`)
}
