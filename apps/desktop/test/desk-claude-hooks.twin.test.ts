import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { afterAll, beforeAll, describe, expect, it } from 'vitest'
import { registerHooksIpc } from '../src/claudeHooks/ipc'
import { HooksManager, HOOKS_STASH_FILE } from '../src/claudeHooks/manager'
import { fakeIpc, HAVE_DESK, startTwin, type Twin } from './desk-twin'

const tmp = mkdtempSync(path.join(os.tmpdir(), 'ccm-desk-hooks-'))
const home = path.join(tmp, 'home')
const root = path.join(home, '.claude')
const dataDir = path.join(tmp, 'data')
const project = path.join(tmp, 'proj')
const userFile = path.join(root, 'settings.json')
const localFile = path.join(project, '.claude', 'settings.local.json')
const plugin = path.join(root, 'plugins', 'cache', 'mk', 'demo', '1.0.0')
const ECHO = "let d='';process.stdin.on('data',c=>d+=c).on('end',()=>{const j=JSON.parse(d);process.stdout.write([j.hook_event_name,j.tool_name,j.cwd===process.env.CLAUDE_PROJECT_DIR].join(' '));process.stderr.write('API_KEY=abc123')})"

function fixture(): void {
  rmSync(tmp, { recursive: true, force: true })
  for (const d of [path.join(project, '.claude'), dataDir, path.join(plugin, 'hooks'), path.join(plugin, '.claude-plugin')]) mkdirSync(d, { recursive: true })
  writeFileSync(
    userFile,
    `{\r\n\t"model": "opus",\r\n\t"hooks": {\r\n\t\t"Stop": [{ "hooks": [{ "type": "command", "command": "echo done", "timeout": 5 }] }, { "matcher": "x", "hooks": [{ "command": "echo two", "type": "command" }] }],\r\n\t\t"PreCompact": [{ "hooks": [{ "type": "command", "command": "node \\"/x/hook-bridge.mjs\\"", "async": true }] }],\r\n\t\t"CustomLater": [{ "hooks": [{ "type": "prompt", "prompt": "Bearer abcdefghijkl" }] }],\r\n\t\t"Another": [{ "hooks": [{ "type": "http", "url": "https://u:p@h.io/x?token=1" }] }]\r\n\t},\r\n\t"enabledPlugins": { "demo@mk": false }\r\n}\r\n`,
  )
  writeFileSync(localFile, JSON.stringify({ hooks: { PreToolUse: [{ matcher: 'Write|Edit', hooks: [{ type: 'command', command: process.execPath, args: ['-e', ECHO] }] }] } }))
  writeFileSync(path.join(plugin, 'hooks', 'hooks.json'), JSON.stringify({ hooks: { SessionStart: [{ hooks: [{ type: 'command', command: 'node "${CLAUDE_PLUGIN_ROOT}/s.js"' }] }] } }))
  writeFileSync(path.join(plugin, '.claude-plugin', 'plugin.json'), JSON.stringify({ name: 'demo', hooks: { UserPromptSubmit: [{ hooks: [{ type: 'http', url: 'https://x' }] }] } }))
  writeFileSync(path.join(root, 'plugins', 'installed_plugins.json'), JSON.stringify({ version: 2, plugins: { 'demo@mk': [{ scope: 'user', installPath: plugin }], 'evil@mk': [{ scope: 'user', installPath: tmp }] } }))
}

function snapshot(): Record<string, string | null> {
  const files = [userFile, `${userFile}.ccm-backup`, localFile, `${localFile}.ccm-backup`, path.join(dataDir, HOOKS_STASH_FILE)]
  return Object.fromEntries(files.map((f) => [path.relative(tmp, f), existsSync(f) ? readFileSync(f, 'utf8').replace(/"at": "[^"]+"/g, '"at": "T"') : null]))
}

const env = { ...process.env, ProgramFiles: process.env.ProgramFiles, LOCALAPPDATA: process.env.LOCALAPPDATA, HOME: home, USERPROFILE: home, CLAUDE_CONFIG_DIR: '' }
let twin: Twin
const node = fakeIpc()
const rust = fakeIpc()
const asks: { side: string; kind: string; req: unknown }[] = []
let answer = true

const manager = (side: string) =>
  new HooksManager({
    env,
    home,
    platform: process.platform,
    dataDir,
    confirmWrite: async (req) => {
      asks.push({ side, kind: 'write', req })
      return answer
    },
    confirmTest: async (req) => {
      asks.push({ side, kind: 'test', req })
      return answer
    },
  })

const norm = (v: unknown): unknown => JSON.parse(JSON.stringify(v).replace(/"ms":\d+/g, '"ms":0'))

describe.skipIf(!HAVE_DESK)('claudeHooks: TS handler and orrery-core desk give the same answers', () => {
  beforeAll(async () => {
    fixture()
    twin = await startTwin({ HOME: home, USERPROFILE: home, CLAUDE_CONFIG_DIR: '', CCM_DATA_DIR: dataDir })
    const gate = (e: unknown): boolean => e === 'app'
    registerHooksIpc({ ipcMain: node.ipcMain, isAppFrame: gate, manager: manager('node') })
    registerHooksIpc({ ipcMain: rust.ipcMain, isAppFrame: gate, manager: manager('rust'), desk: twin.router })
  }, 30_000)

  afterAll(async () => {
    await twin?.host.stop()
    rmSync(tmp, { recursive: true, force: true })
  })

  const both = async (steps: (call: (ch: string, ...args: unknown[]) => Promise<unknown>) => Promise<unknown>) => {
    fixture()
    asks.length = 0
    const a = norm(await steps((ch, ...args) => node.call(ch, 'app', ...args)))
    const fa = snapshot()
    const askA = asks.filter((x) => x.side === 'node').map((x) => [x.kind, x.req])
    fixture()
    const b = norm(await steps((ch, ...args) => rust.call(ch, 'app', ...args)))
    const fb = snapshot()
    const askB = asks.filter((x) => x.side === 'rust').map((x) => [x.kind, x.req])
    expect(b).toEqual(a)
    expect(fb).toEqual(fa)
    expect(askB).toEqual(askA)
    return a
  }

  type Row = { id: string; event: string; source: string }
  const rows = (r: unknown): Row[] => (r as { rows: Row[] }).rows

  it('lists the same rows with and without a project, and refuses bad dirs', async () => {
    const a = (await both(async (call) => [await call('ccm:hooks-list', project), await call('ccm:hooks-list', null), await call('ccm:hooks-list', 'rel/dir'), await call('ccm:hooks-list', 5)])) as unknown[]
    expect(rows(a[0]).length).toBeGreaterThan(5)
    expect(JSON.stringify(a[0])).toContain('••••')
  })

  it('turns hooks off and on with the same files, stash, backup and confirm text', async () => {
    await both(async (call) => {
      const list = rows(await call('ccm:hooks-list', project))
      const stop = list.find((r) => r.event === 'Stop')!
      const local = list.find((r) => r.source === 'local')!
      const out = [await call('ccm:hooks-toggle', { id: stop.id, dir: project, on: false })]
      out.push(await call('ccm:hooks-toggle', { id: local.id, dir: project, on: false }))
      out.push(await call('ccm:hooks-list', project))
      out.push(await call('ccm:hooks-toggle', { id: stop.id, dir: project, on: true }))
      out.push(await call('ccm:hooks-list', project))
      return out
    })
  })

  it('refuses orrery, plugin, bad ids, cancels and busy-free errors the same way', async () => {
    await both(async (call) => {
      const list = rows(await call('ccm:hooks-list', project))
      const bridge = list.find((r) => r.event === 'PreCompact')!
      const plug = list.find((r) => r.source === 'plugin')!
      const out = [
        await call('ccm:hooks-toggle', { id: bridge.id, dir: project, on: false }),
        await call('ccm:hooks-toggle', { id: plug.id, dir: project, on: false }),
        await call('ccm:hooks-toggle', { id: '../x', dir: project, on: false }),
        await call('ccm:hooks-toggle', { id: list[0]!.id, dir: project, on: 'no' }),
        await call('ccm:hooks-toggle', { id: 'a'.repeat(24), dir: project, on: true }),
        await call('ccm:hooks-toggle', null),
      ]
      answer = false
      out.push(await call('ccm:hooks-toggle', { id: list.find((r) => r.event === 'Stop')!.id, dir: project, on: false }))
      out.push(await call('ccm:hooks-test', { id: list.find((r) => r.source === 'local')!.id, dir: project }))
      answer = true
      return out
    })
  })

  it('test-runs a hook with the same payload, env and masked output', async () => {
    const a = (await both(async (call) => {
      const list = rows(await call('ccm:hooks-list', project))
      const local = list.find((r) => r.source === 'local')!
      const stop = list.find((r) => r.event === 'Stop')!
      const plug = list.find((r) => r.event === 'UserPromptSubmit')!
      return [
        await call('ccm:hooks-test', { id: local.id, dir: project }),
        await call('ccm:hooks-test', { id: local.id, dir: project }),
        await call('ccm:hooks-test', { id: stop.id, dir: project }),
        await call('ccm:hooks-test', { id: plug.id, dir: project }),
        await call('ccm:hooks-test', { id: 'b'.repeat(24) }),
      ]
    })) as { stdout?: string; stderr?: string }[]
    expect(a[0]!.stdout).toBe('PreToolUse Write true')
    expect(a[0]!.stderr).toBe('API_KEY=••••')
    expect(twin.logs).toEqual([])
    expect(twin.fellBack).toEqual([])
  }, 60_000)

  it('other frames get the TS refusal without reaching the desk', async () => {
    expect(await rust.call('ccm:hooks-list', 'guest', project)).toEqual({ ok: false, error: 'unavailable' })
    expect(await rust.call('ccm:hooks-toggle', 'guest', {})).toEqual({ ok: false, error: 'invalid' })
  })
})
