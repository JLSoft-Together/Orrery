import { chmodSync, existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { afterAll, beforeAll, describe, expect, it } from 'vitest'
import { registerDeepSeekIpc, SYNC_RECORD, type DeepSeekConfirm, type DeepSeekEvent } from '../src/deepseek/ipc'
import { fakeIpc, HAVE_DESK, listTree, startTwin, swapRoot, type Twin } from './desk-twin'

const tmp = mkdtempSync(path.join(os.tmpdir(), 'ccm-desk-ds-'))

const CODEWHALE = `#!/bin/sh
S="$HOME/.cwstate"
mkdir -p "$S"
echo "$@" >> "$S/calls.log"
case "$1" in
  --version) echo "codewhale 0.10.2 (abc)";;
  update) echo "updating codewhale"; echo "ghp_abcdefghijklmnopqrstuvwxyz0123456789 $HOME";;
  auth)
    case "$2" in
      status)
        echo "provider       config   keyring    env      status"
        if [ -f "$S/key" ]; then echo "deepseek       set      -          -        ok *"; else echo "deepseek       -        -          -        unset *"; fi;;
      set) cat > "$S/key"; if grep -q boom "$S/key"; then echo "bad key $(cat "$S/key") in $HOME" >&2; exit 3; fi;;
      clear) cat > "$S/answer"; rm -f "$S/key";;
    esac;;
  config)
    if [ "$2" = get ]; then echo "Usage reporting: $(cat "$S/tele" 2>/dev/null || echo On) (default)"; elif [ "$4" = true ]; then echo On > "$S/tele"; else echo Off > "$S/tele"; fi;;
  doctor) printf 'Doctor\\n  ok\\nUpdates:\\n  - current: v0.10.2\\n  - latest: v0.11.0\\n';;
esac
`

const NPM = (src: string, bin: string): string => `#!/bin/sh
echo "added 1 package in 2s"
cp "${src}" "${bin}/codewhale"
chmod +x "${bin}/codewhale"
`

interface Side {
  home: string
  bin: string
  data: string
  ipc: ReturnType<typeof fakeIpc>
  confirms: DeepSeekConfirm[]
  events: DeepSeekEvent[]
}

function layout(name: string): Side {
  const root = path.join(tmp, name)
  const home = path.join(root, 'home')
  const bin = path.join(root, 'bin')
  const data = path.join(root, 'data')
  for (const d of [home, bin, data, path.join(home, '.claude')]) mkdirSync(d, { recursive: true })
  const src = path.join(root, 'codewhale.src')
  writeFileSync(src, CODEWHALE)
  writeFileSync(path.join(bin, 'npm'), NPM(src, bin))
  chmodSync(path.join(bin, 'npm'), 0o755)
  writeFileSync(path.join(home, '.claude', 'CLAUDE.md'), 'Rule one\n@extra.md\nAt the end\n')
  writeFileSync(path.join(home, '.claude', 'extra.md'), 'Extra rule\n')
  writeFileSync(path.join(home, '.claude.json'), JSON.stringify({ mcpServers: { figma: { type: 'http', url: 'https://f.example/mcp', headers: { a: 'b' } }, local: { command: 'npx', args: ['-y', 'x'], env: { K: 'v' } }, orrery: { command: 'x' } } }))
  mkdirSync(path.join(home, '.codewhale'), { recursive: true })
  writeFileSync(path.join(home, '.codewhale', 'AGENTS.md'), 'My own notes\n')
  return { home, bin, data, ipc: fakeIpc(), confirms: [], events: [] }
}

const baseEnv = (side: Side): Record<string, string> => ({ HOME: side.home, USERPROFILE: side.home, CLAUDE_CONFIG_DIR: '', CODEX_HOME: '', PATH: `${side.bin}${path.delimiter}${process.env.PATH ?? ''}` })

let twin: Twin
let node: Side
let rust: Side
let answer = true
const gate = (e: unknown): boolean => e === 'app'
const eventsOf = (side: Side): DeepSeekEvent[] => (side === rust ? twin.events.filter((e) => e.event === 'ccm:deepseek-event').map((e) => e.payload as DeepSeekEvent) : side.events)

const norm = (v: unknown): unknown => swapRoot(JSON.parse(JSON.stringify(v).replace(/"syncedAt":\d+(\.\d+)?/g, '"syncedAt":0').replace(/"at":\d+(\.\d+)?/g, '"at":0')), rust.home, node.home)

describe.skipIf(!HAVE_DESK || process.platform === 'win32')('deepseek: TS handler and orrery-core desk give the same answers', () => {
  beforeAll(async () => {
    node = layout('node')
    rust = layout('rust')
    const env = { ...process.env, ...baseEnv(node) } as NodeJS.ProcessEnv
    twin = await startTwin({ ...baseEnv(rust), CCM_DATA_DIR: rust.data, CCM_DEEPSEEK_FEATURE: '1' })
    const confirm = (side: Side) => async (_e: unknown, plan: DeepSeekConfirm) => {
      side.confirms.push(plan)
      return answer
    }
    registerDeepSeekIpc({ ipcMain: node.ipc.ipcMain, isAppFrame: gate, confirm: confirm(node) as never, emit: (e) => node.events.push(e), dataDir: node.data, env, home: node.home })
    registerDeepSeekIpc({ ipcMain: rust.ipc.ipcMain, isAppFrame: gate, confirm: confirm(rust) as never, emit: (e) => rust.events.push(e), dataDir: rust.data, env: { ...process.env, ...baseEnv(rust) } as NodeJS.ProcessEnv, home: rust.home, desk: twin.router })
  }, 30_000)

  afterAll(async () => {
    await twin?.host.stop()
    rmSync(tmp, { recursive: true, force: true })
  })

  const both = async (channel: string, event: unknown = 'app', ...args: unknown[]) => {
    const a = norm(await node.ipc.call(channel, event, ...args))
    const b = norm(await rust.ipc.call(channel, event, ...args))
    expect(b).toEqual(a)
    return a as Record<string, unknown>
  }

  it('install through npm: confirm, lines and result match', async () => {
    expect(twin.host.channels).toContain('ccm:deepseek-install')
    const before = await both('ccm:deepseek-status', 'app', true)
    expect(before).toMatchObject({ installed: false, installRoute: 'npm', sync: 'never' })
    answer = false
    expect(await both('ccm:deepseek-install')).toEqual({ ok: false, code: 'cancelled' })
    answer = true
    expect(await both('ccm:deepseek-install')).toEqual({ ok: true })
    expect(await both('ccm:deepseek-install')).toEqual({ ok: false, code: 'installed' })
    expect(norm(rust.confirms)).toEqual(norm(node.confirms))
    expect(node.confirms[1]).toEqual({ kind: 'install', commands: ['npm i -g codewhale', 'codewhale --version'] })
    const lines = (side: Side) => eventsOf(side).map((e) => (e.phase === 'install' ? e.line : '')).filter(Boolean)
    expect(lines(rust)).toEqual(lines(node))
    expect(lines(node)).toContain('added 1 package in 2s')
  })

  it('status, login, telemetry, update check and logout match; the key never comes back', async () => {
    const s = await both('ccm:deepseek-status', 'app', true)
    expect(s).toMatchObject({ installed: true, version: '0.10.2', loggedIn: false, telemetry: true, origin: 'standalone' })
    expect(await both('ccm:deepseek-login', 'app', 'short')).toEqual({ ok: false, code: 'badInput' })
    expect(await both('ccm:deepseek-login', 'app', 42)).toEqual({ ok: false, code: 'badInput' })
    const bad = await both('ccm:deepseek-login', 'app', 'sk-boom-1234567890')
    expect(bad['code']).toBe('failed')
    expect(JSON.stringify(bad)).not.toContain('sk-boom-1234567890')
    expect(await both('ccm:deepseek-login', 'app', ' sk-good-1234567890 ')).toEqual({ ok: true })
    expect((await both('ccm:deepseek-status', 'app', true))['loggedIn']).toBe(true)
    expect(await both('ccm:deepseek-telemetry', 'app', 'x')).toEqual({ ok: false, code: 'badInput' })
    expect(await both('ccm:deepseek-telemetry', 'app', false)).toEqual({ ok: true })
    expect((await both('ccm:deepseek-status', 'app', true))['telemetry']).toBe(false)
    expect(await both('ccm:deepseek-check-update')).toEqual({ ok: true, current: '0.10.2', latest: '0.11.0', newer: true })
    answer = false
    expect(await both('ccm:deepseek-logout')).toEqual({ ok: false, code: 'cancelled' })
    answer = true
    expect(await both('ccm:deepseek-logout')).toEqual({ ok: true })
    expect((await both('ccm:deepseek-status', 'app', true))['loggedIn']).toBe(false)
    for (const side of [node, rust]) {
      const calls = readFileSync(path.join(side.home, '.cwstate', 'calls.log'), 'utf8')
      expect(calls).toContain('auth set --provider deepseek --api-key-stdin')
      expect(calls).not.toContain('sk-good')
    }
    expect(readFileSync(path.join(rust.home, '.cwstate', 'answer'), 'utf8')).toBe(readFileSync(path.join(node.home, '.cwstate', 'answer'), 'utf8'))
  })

  it('update runs the planned step and masks secrets in streamed lines', async () => {
    answer = false
    expect(await both('ccm:deepseek-update')).toEqual({ ok: false, code: 'cancelled' })
    answer = true
    expect(await both('ccm:deepseek-update')).toEqual({ ok: true })
    expect(norm(rust.confirms.at(-1))).toEqual(norm(node.confirms.at(-1)))
    expect(node.confirms.at(-1)).toEqual({ kind: 'update', commands: ['codewhale update'] })
    const lines = (side: Side) => eventsOf(side).map((e) => (e.phase === 'install' ? e.line : '')).filter(Boolean)
    expect(norm(lines(rust))).toEqual(norm(lines(node)))
    expect(lines(rust).join('\n')).not.toContain('ghp_abcdefghijklmnopqrstuvwxyz0123456789')
    expect(await both('ccm:deepseek-cancel')).toBe(false)
  })

  it('sync writes the same files, keeps a backup and goes current', async () => {
    answer = false
    expect(await both('ccm:deepseek-sync')).toEqual({ ok: false, code: 'cancelled' })
    expect(existsSync(path.join(rust.home, '.codewhale', 'config.toml'))).toBe(false)
    answer = true
    const first = await both('ccm:deepseek-sync')
    expect(first['ok']).toBe(true)
    expect(first['mcpAdded']).toEqual(['figma', 'local'])
    expect(norm(rust.confirms.at(-1))).toEqual(norm(node.confirms.at(-1)))
    const trees = [node, rust].map((s) => ({
      codewhale: listTree(path.join(s.home, '.codewhale')),
      claude: listTree(path.join(s.home, '.claude', 'skills')),
      codex: listTree(path.join(s.home, '.codex', 'skills')),
      record: JSON.parse(readFileSync(path.join(s.data, SYNC_RECORD), 'utf8')) as { at: number; hash: string },
    }))
    const strip = (t: (typeof trees)[number]) => {
      const { record, codewhale, ...rest } = t
      const cw = Object.fromEntries(Object.entries(codewhale).filter(([k]) => !k.startsWith('.cwstate') && !k.includes('/.cwstate')))
      return { ...rest, codewhale: cw, hash: record.hash }
    }
    expect(strip(trees[1]!)).toEqual(strip(trees[0]!))
    expect(trees[1]!.codewhale['AGENTS.md.orrery-backup']).toBe('My own notes\n')
    expect(typeof trees[1]!.record.at).toBe('number')
    expect((await both('ccm:deepseek-status', 'app', true))['sync']).toBe('current')
    const second = await both('ccm:deepseek-sync')
    expect((second['files'] as { action: string }[]).every((f) => f.action === 'unchanged')).toBe(true)
    writeFileSync(path.join(rust.home, '.claude', 'extra.md'), 'Changed rule\n')
    writeFileSync(path.join(node.home, '.claude', 'extra.md'), 'Changed rule\n')
    expect((await both('ccm:deepseek-status', 'app', true))['sync']).toBe('stale')
  })

  it('other frames get the same refusal and the desk never fell back', async () => {
    for (const ch of ['ccm:deepseek-status', 'ccm:deepseek-install', 'ccm:deepseek-login', 'ccm:deepseek-logout', 'ccm:deepseek-telemetry', 'ccm:deepseek-sync', 'ccm:deepseek-check-update', 'ccm:deepseek-update', 'ccm:deepseek-cancel']) {
      await both(ch, 'guest', 'sk-good-1234567890')
    }
    expect(twin.logs).toEqual([])
    expect(twin.fellBack).toEqual([])
  })
})
