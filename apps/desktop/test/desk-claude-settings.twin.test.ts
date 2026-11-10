import { execFileSync } from 'node:child_process'
import { existsSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { afterAll, beforeAll, describe, expect, it } from 'vitest'
import { BRIDGE_EXE, installRustBridge, migrateBridges, rustBridgeCommand } from '../src/bridgeRust'
import { registerClaudeSettingsIpc, type HooksConfirm, type StatusConfirm } from '../src/claudeSettings'
import { DESK_BIN, fakeIpc, HAVE_DESK, startTwin, type Twin } from './desk-twin'

const REPO = path.resolve(__dirname, '..', '..', '..')
const BRIDGE_BIN = path.join(path.dirname(DESK_BIN), BRIDGE_EXE)
const HAVE_BRIDGE = HAVE_DESK && existsSync(BRIDGE_BIN)
const tmp = mkdtempSync(path.join(os.tmpdir(), 'ccm-desk-settings-'))
const home = path.join(tmp, 'home')
const dataDir = path.join(tmp, 'data')
const settings = path.join(home, '.claude', 'settings.json')
const STATUS = 'node "C:/x/statusline-bridge.mjs"'
const HOOK = 'node "C:/x/hook-bridge.mjs"'

let twin: Twin
const node = fakeIpc()
const rust = fakeIpc()
const asks: { side: string; req: StatusConfirm | HooksConfirm }[] = []
let answer = true
let commands: { status: string | null; hook: string | null } = { status: STATUS, hook: HOOK }

function reset(text: string | null): void {
  rmSync(home, { recursive: true, force: true })
  mkdirSync(path.dirname(settings), { recursive: true })
  if (text !== null) writeFileSync(settings, text)
}

const read = (f: string): string | null => (existsSync(f) ? readFileSync(f, 'utf8') : null)

describe.skipIf(!HAVE_DESK)('status line + hook bridge settings: TS and orrery-core desk match', () => {
  beforeAll(async () => {
    reset(null)
    twin = await startTwin({ HOME: home, USERPROFILE: home, CLAUDE_CONFIG_DIR: '', CCM_DATA_DIR: dataDir })
    const deps = (side: string) => ({
      isAppFrame: (e: unknown): boolean => e === 'app',
      settingsFile: () => settings,
      statusCommand: () => commands.status,
      hookCommand: () => commands.hook,
      confirmStatus: async (_e: unknown, req: StatusConfirm) => (asks.push({ side, req }), answer),
      confirmHooks: async (_e: unknown, req: HooksConfirm) => (asks.push({ side, req }), answer),
    })
    registerClaudeSettingsIpc({ ipcMain: node.ipcMain, ...deps('node') })
    registerClaudeSettingsIpc({ ipcMain: rust.ipcMain, ...deps('rust'), desk: twin.router })
  }, 30_000)

  afterAll(async () => {
    await twin?.host.stop()
    rmSync(tmp, { recursive: true, force: true })
  })

  const CASES: (string | null)[] = [
    null,
    '',
    '{ "theme": "dark" }',
    'not json',
    '[1]',
    '{\n  "statusLine": { "type": "command", "command": "ccstatus --x", "padding": 0 },\n  "hooks": { "Stop": [{ "hooks": [{ "type": "command", "command": "say" }] }], "PreCompact": [{ "hooks": [{ "command": "node \\"/old/hook-bridge.mjs\\"" }] }] }\n}\n',
    '{ "statusLine": { "command": "node statusline-bridge.mjs" }, "hooks": { "PreCompact": [{"hooks":[{"command":"hook-bridge"}]}], "PostCompact": [{"hooks":[{"command":"hook-bridge"}]}], "PermissionDenied": [{"hooks":[{"command":"hook-bridge"}]}], "PermissionRequest": [{"hooks":[{"command":"hook-bridge"}]}] } }',
  ]

  const run = async (call: (ch: string, ...a: unknown[]) => Promise<unknown>, text: string | null) => {
    const out: unknown[] = []
    for (const step of [
      () => call('ccm:statusline-state'),
      () => call('ccm:statusline-enable'),
      () => call('ccm:hooks-state'),
      () => call('ccm:hooks-set', true),
      () => call('ccm:hooks-set', false),
      () => call('ccm:hooks-set', 'yes'),
    ]) {
      reset(text)
      out.push(await step(), read(settings), read(`${settings}.ccm-backup`))
    }
    return out
  }

  it('state, enable and hooks set give the same results, files and confirm text', async () => {
    for (const ans of [true, false]) {
      answer = ans
      for (const text of CASES) {
        asks.length = 0
        const a = await run((ch, ...args) => node.call(ch, 'app', ...args), text)
        const b = await run((ch, ...args) => rust.call(ch, 'app', ...args), text)
        expect(b).toEqual(a)
        expect(asks.filter((x) => x.side === 'rust').map((x) => x.req)).toEqual(asks.filter((x) => x.side === 'node').map((x) => x.req))
      }
    }
    answer = true
    commands = { status: null, hook: null }
    reset('{}')
    expect(await rust.call('ccm:statusline-enable', 'app')).toEqual(await node.call('ccm:statusline-enable', 'app'))
    expect(await rust.call('ccm:hooks-set', 'app', true)).toEqual(await node.call('ccm:hooks-set', 'app', true))
    commands = { status: STATUS, hook: HOOK }
    expect(twin.logs).toEqual([])
  })

  it('moves old bridge entries to orrery-bridge the same way', async () => {
    const bin = path.join(dataDir, 'bin').replace(/\\/g, '/')
    const upper = process.platform === 'win32' ? bin.toUpperCase() : bin
    const next = { statusline: '"N/orrery-bridge.exe" statusline-bridge', hook: '"N/orrery-bridge.exe" hook-bridge' }
    const texts = [
      null,
      'oops',
      '{}',
      `{\r\n    "statusLine": { "type": "command", "command": "node \\"${bin}/statusline-bridge.mjs\\" --chain-b64 eA==" },\r\n    "hooks": { "PreCompact": [{ "hooks": [{ "type": "command", "command": "\\"${upper}/hook-bridge.cmd\\"", "async": true }, { "command": "node \\"${bin}/hook-bridge.mjsx\\"" }] }], "Stop": [{ "hooks": [{ "command": "node \\"${bin}/hook-bridge.mjs\\"" }] }] }\r\n}\r\n`,
      `{ "statusLine": { "command": "\\"${bin}/statusline-bridge.sh\\"" } }`,
    ]
    for (const text of texts) {
      reset(text)
      const a = migrateBridges(settings, dataDir, next)
      const fa = [read(settings), read(`${settings}.ccm-backup`)]
      reset(text)
      const b = await twin.router.route('bridge', 'desk:bridge-migrate', () => true, () => null)(null, next)
      expect(b).toEqual(a)
      expect([read(settings), read(`${settings}.ccm-backup`)]).toEqual(fa)
    }
    expect(twin.logs).toEqual([])
  })

  it.skipIf(!HAVE_BRIDGE)('installs orrery-bridge to the same place with the same commands', async () => {
    const a = installRustBridge(BRIDGE_BIN, path.join(tmp, 'n'))
    const res = (await twin.host.call('desk:bridge-install', [{ source: BRIDGE_BIN }])) as { ok: boolean; statusline: { script: string; command: string } }
    expect(res.ok).toBe(true)
    expect(a).toBe(path.join(tmp, 'n', 'bin', BRIDGE_EXE))
    expect(res.statusline).toEqual(rustBridgeCommand(path.join(dataDir, 'bin', BRIDGE_EXE), 'statusline-bridge'))
    expect(readFileSync(res.statusline.script).equals(readFileSync(a))).toBe(true)
  })
})

const SID = '0123abcd-0000-4000-8000-abcdef012345'

describe.skipIf(!HAVE_BRIDGE)('orrery-bridge writes what the node bridges write', () => {
  const run = (side: 'node' | 'rust', mode: 'statusline-bridge' | 'hook-bridge', input: string, dir: string, extra: string[] = []): string => {
    const env = { ...process.env, CCM_DATA_DIR: dir, CLAUDE_CODE_AUTO_COMPACT_WINDOW: '200000' }
    const opts = { input, env, encoding: 'utf8' as const }
    return side === 'node' ? execFileSync(process.execPath, [path.join(REPO, 'scripts', `${mode}.mjs`), ...extra], opts) : execFileSync(BRIDGE_BIN, [mode, ...extra], opts)
  }
  const tree = (dir: string): Record<string, string> => {
    const out: Record<string, string> = {}
    for (const sub of ['statusline', 'hooks']) {
      const d = path.join(dir, sub)
      if (!existsSync(d)) continue
      for (const f of readdirSync(d)) out[`${sub}/${f}`] = readFileSync(path.join(d, f), 'utf8').replace(/\d{4}-\d\d-\d\dT[\d:.]+Z/g, 'T')
    }
    return out
  }

  it('status line records, default lines and chained commands match', () => {
    const inputs = [
      JSON.stringify({ session_id: SID, model: { display_name: 'Opus' }, cost: { total_cost_usd: 0.125 }, context_window: { context_window_size: 200000, used_percentage: 12.5, total_tokens: 0 }, rate_limits: { five_hour: { used_percentage: 120.4, resets_at: 1700000000 }, seven_day: { used_percentage: 3 } } }),
      JSON.stringify({ session_id: 'nope', model: {} }),
      'not json',
      '',
    ]
    for (const input of inputs) {
      const [a, b] = (['node', 'rust'] as const).map((side) => {
        const dir = path.join(tmp, `sl-${side}`)
        rmSync(dir, { recursive: true, force: true })
        return { out: run(side, 'statusline-bridge', input, dir), files: tree(dir) }
      })
      expect(b).toEqual(a)
    }
    const chain = Buffer.from('node -e "process.stdin.pipe(process.stdout)"').toString('base64')
    const input = JSON.stringify({ session_id: SID })
    expect(run('rust', 'statusline-bridge', input, path.join(tmp, 'c1'), ['--chain-b64', chain])).toBe(run('node', 'statusline-bridge', input, path.join(tmp, 'c2'), ['--chain-b64', chain]))
  })

  it('hook records and permission asks match', () => {
    const events = [
      { session_id: SID, hook_event_name: 'PreCompact', trigger: 'manual' },
      { session_id: SID, hook_event_name: 'PermissionDenied', tool_name: 'Bash' },
      { session_id: SID, hook_event_name: 'PermissionDenied', tool_name: 'bad tool!' },
      { session_id: SID, hook_event_name: 'PostCompact' },
      { session_id: SID, hook_event_name: 'PermissionRequest', tool_name: 'Bash', cwd: '/p', tool_input: { command: 'git push --force' }, permission_suggestions: [{ type: 'addRules', behavior: 'allow', rules: [{ toolName: 'Bash', ruleContent: 'git push:*' }, { toolName: 'Read' }] }] },
      { session_id: SID, hook_event_name: 'PermissionRequest', tool_name: 'Read', cwd: '' },
      { session_id: 'x', hook_event_name: 'PreCompact' },
    ]
    const [a, b] = (['node', 'rust'] as const).map((side) => {
      const dir = path.join(tmp, `hk-${side}`)
      rmSync(dir, { recursive: true, force: true })
      const outs = events.map((e) => run(side, 'hook-bridge', JSON.stringify(e), dir))
      return { outs, files: tree(dir) }
    })
    expect(b).toEqual(a)
    expect(Object.keys(a!.files)).toContain('hooks/permission-asks.jsonl')
  })
})
