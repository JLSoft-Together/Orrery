// Runs every workspace E2E script in turn against the Tauri shell (apps/shell/target/debug/orrery-shell, or the packaged exe given as argv[2]); --shell=electron or CCM_E2E_SHELL=electron runs the Electron dev build instead.
// Build first: npm run build -w @ccm/web && node scripts/gen-desk-shim.mjs && (cd apps/shell && cargo build -j 4); Electron also needs npm run build -w @ccm/desktop. Spawns real shells, and p2, v25 and shell-agent start `claude` in this repo (shell-agent also `codex`) (no prompt is sent; v25 puts an image on the clipboard).
const { spawnSync } = require('node:child_process')
const fs = require('node:fs')
const path = require('node:path')
const { OUT } = require('./cdp.cjs')
const CLASSIC_ONLY = new Set(['v25', 'tier2', 'extensions', 'push', 'split-diff', 'v70-tools'])
const ELECTRON_ONLY = new Set(['v74', 'v75', 'v76b', 'v77a'])
let failed = 0
for (const name of ['p2', 'p3', 'p4', 'p5', 'p6', 'csp', 'desktop-only', 'split', 'browser', 'tabs-cwd', 'tier2', 'explorer', 'v25', 'minimap', 'v27', 'extensions', 'diff', 'log', 'sidebar', 'rail', 'explorer-menu', 'split-diff', 'flutter-preview', 'run-pane', 'drawable-preview', 'file-editor', 'loose-kit', 'layout-speed', 'agent-map', 'pane-drag', 'project-panes', 'v49-open', 'v50-groups', 'v51-compact', 'v52-polish', 'push', 'links', 'devices', 'v67-ide', 'v69-ide', 'v70-tools', 'whats-new', 'v72', 'v73', 'v74', 'v75', 'v76', 'v76b', 'v77a', 'v77b', 'v78', 'v80-mcp', 'v80-toasts', 'agent-api', 'shell-agent', 'v79-shots', 'v104-deepseek-lazy', 'v104-clash', 'v104-inbox', 'v104-turn-changes']) {
  const res = spawnSync(process.execPath, [path.join(__dirname, `${name}.cjs`), ...process.argv.slice(2)], { stdio: ['ignore', 'pipe', 'inherit'], encoding: 'utf8', env: { ...process.env, ...(CLASSIC_ONLY.has(name) ? { CCM_E2E_UI: 'classic' } : {}), ...(ELECTRON_ONLY.has(name) ? { CCM_E2E_SHELL: 'electron' } : {}) } })
  process.stdout.write(`\n# ${name}\n${res.stdout}`)
  failed += (res.stdout.match(/^FAIL /gm) ?? []).length
}
console.log(`\n${failed ? failed + ' failed' : 'all passed'} — screenshots in ${OUT}`)
process.exit(failed ? 1 : 0)
