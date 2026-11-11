import { copyFileSync, linkSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { afterAll, beforeAll, describe, expect, it } from 'vitest'
import { CATALOG } from '@ccm/shared'
import { registerExtensionsIpc, type ConfirmPlan } from '../src/extensions/ipc'
import { fakeIpc, HAVE_DESK, startTwin, type Twin } from './desk-twin'

const tmp = mkdtempSync(path.join(os.tmpdir(), 'ccm-desk-ext-'))
const home = path.join(tmp, 'home')
const project = path.join(tmp, 'proj')
const bin = path.join(tmp, 'bin')
const log = path.join(tmp, 'calls.log')
const CLI = process.platform === 'win32' ? 'claude.exe' : 'claude'

const FAKE = `const fs = require('fs')
const args = [require('path').basename(process.argv[1]), ...process.argv.slice(2)]
fs.appendFileSync(${JSON.stringify(log)}, JSON.stringify({ cwd: process.cwd(), args }) + '\\n')
const cmd = args.slice(0, 3).join(' ')
const out = (s) => process.stdout.write(s)
if (cmd === 'plugin list --json') out(JSON.stringify([{ id: 'a@m', scope: 'user', enabled: true }, { id: 'b@m', scope: 'project', projectPath: ${JSON.stringify(project)} }, { id: 'c@m', scope: 'local', projectPath: '/other', enabled: false }, { id: 5 }]))
else if (cmd === 'plugin marketplace list') out(JSON.stringify([{ name: 'claude-plugins-official' }, { nope: 1 }]))
else if (cmd === 'plugin list --available') out(JSON.stringify({ available: [{ pluginId: 'p1@cool-market', name: 'p1', marketplaceName: 'cool-market', description: 'x'.repeat(400) }, { pluginId: 'q@other', name: 'q', marketplaceName: 'other' }] }))
else if (cmd === 'mcp list') out('Checking MCP server health...\\n\\nfigma: https://mcp.figma.com/mcp (HTTP) - ✔ Connected\\nlocalonly: npx x - ✘ Failed to connect\\nplugin:gh:gh: https://g - Needs authentication\\n')
else if (cmd.startsWith('plugin marketplace add')) {
  if (args[3].includes('dup')) { process.stderr.write('Marketplace already exists'); process.exit(1) }
  out('Adding...\\n' + JSON.stringify({ name: 'cool-market' }))
} else if (cmd.startsWith('plugin install') && args[2].startsWith('term')) { process.stderr.write('This plugin requires --yes in a TTY'); process.exit(1) }
else if (cmd.startsWith('mcp remove')) { process.stderr.write('No MCP server named ' + args[2] + ' token=ghp_abcdefghijklmnop1234 at ' + ${JSON.stringify(home)}); process.exit(2) }
else out('ok')
`

function fixture(): void {
  rmSync(tmp, { recursive: true, force: true })
  for (const d of [bin, path.join(home, '.claude', 'skills', 'b-skill'), path.join(home, '.claude', 'skills', 'a-skill'), path.join(home, '.claude', 'skills', '.hidden'), path.join(project, '.claude', 'skills', 'p')]) mkdirSync(d, { recursive: true })
  if (process.platform !== 'win32') {
    writeFileSync(path.join(bin, CLI), `#!/bin/sh\nexec ${JSON.stringify(process.execPath)} "$@"\n`, { mode: 0o755 })
  } else {
    try {
      linkSync(process.execPath, path.join(bin, CLI))
    } catch {
      copyFileSync(process.execPath, path.join(bin, CLI))
    }
  }
  for (const dir of [home, project]) for (const name of ['plugin', 'mcp']) writeFileSync(path.join(dir, name), FAKE)
  writeFileSync(path.join(home, '.claude.json'), JSON.stringify({ mcpServers: { figma: {}, shared: {} }, projects: { [project.replace(/\\/g, '/')]: { mcpServers: { localonly: {}, shared: {} } } } }))
  writeFileSync(path.join(project, '.mcp.json'), JSON.stringify({ mcpServers: { teamserver: {} } }))
}

const pathKey = Object.keys(process.env).find((k) => k.toUpperCase() === 'PATH') ?? 'PATH'
const overrides = { HOME: home, USERPROFILE: home, CLAUDE_CONFIG_DIR: '', [pathKey]: `${bin}${path.delimiter}${process.env[pathKey] ?? ''}` }
const env = { ...process.env, ...overrides }
let twin: Twin
const node = fakeIpc()
const rust = fakeIpc()
const plans: { side: string; plan: ConfirmPlan }[] = []
let answer = true

const norm = (v: unknown): unknown => JSON.parse(JSON.stringify(v).replace(/"at":\d+/g, '"at":0'))
const calls = (): unknown[] =>
  readFileSync(log, 'utf8')
    .split('\n')
    .filter(Boolean)
    .sort()
    .map((l) => JSON.parse(l) as unknown)

describe.skipIf(!HAVE_DESK)('extensions: TS handler and orrery-core desk give the same answers', () => {
  beforeAll(async () => {
    fixture()
    twin = await startTwin(overrides)
    const gate = (e: unknown): boolean => e === 'app'
    const confirm = (side: string) => async (_e: unknown, plan: ConfirmPlan) => {
      plans.push({ side, plan })
      return answer
    }
    registerExtensionsIpc({ ipcMain: node.ipcMain, isAppFrame: gate, confirm: confirm('node'), env, home, settleMs: 10 })
    registerExtensionsIpc({ ipcMain: rust.ipcMain, isAppFrame: gate, confirm: confirm('rust'), env, home, desk: twin.router, settleMs: 10 })
  }, 30_000)

  afterAll(async () => {
    await twin?.host.stop()
    rmSync(tmp, { recursive: true, force: true })
  })

  const both = async (steps: (call: (ch: string, ...args: unknown[]) => Promise<unknown>) => Promise<unknown>) => {
    fixture()
    plans.length = 0
    const a = norm(await steps((ch, ...args) => node.call(ch, 'app', ...args)))
    const ca = calls()
    fixture()
    const b = norm(await steps((ch, ...args) => rust.call(ch, 'app', ...args)))
    const cb = calls()
    expect(b).toEqual(a)
    expect(cb).toEqual(ca)
    expect(plans.filter((p) => p.side === 'rust').map((p) => p.plan)).toEqual(plans.filter((p) => p.side === 'node').map((p) => p.plan))
    return a
  }

  it('status and mcp list match, cached until fresh', async () => {
    const a = (await both(async (call) => [
      await call('ccm:ext-status', null),
      await call('ccm:ext-status', project),
      await call('ccm:ext-status', 'not/abs'),
      await call('ccm:ext-mcp', project),
      await call('ccm:ext-mcp', project),
      await call('ccm:ext-mcp', project, true),
      await call('ccm:ext-mcp', null),
    ])) as unknown[]
    expect(JSON.stringify(a[1])).toContain('teamserver')
    expect(JSON.stringify(a[3])).toContain('needsAuth')
  }, 60_000)

  it('installs, removes and adds sources with the same argv, consent and errors', async () => {
    const a = (await both(async (call) => [
      await call('ccm:ext-install', { id: 'figma', values: {}, scope: 'user' }),
      await call('ccm:ext-install', { id: 'github', values: { GITHUB_PAT: 'ghp_secretsecretsecret' }, scope: 'local', dir: project }),
      await call('ccm:ext-install', { id: 'meshy', values: { MESHY_API_KEY: 'abcdefgh1234' }, scope: 'project', dir: project }),
      await call('ccm:ext-install', { id: 'pm-skills', values: {}, scope: 'user', plugins: ['pm-toolkit', 'pm-execution'] }),
      await call('ccm:ext-install', { id: 'frontend-design', values: {}, scope: 'user' }),
      await call('ccm:ext-install', { id: 'github', values: { GITHUB_PAT: 'short' }, scope: 'user' }),
      await call('ccm:ext-install', { id: 'gitlab', values: { GITLAB_HOST: ' ' }, scope: 'project' }),
      await call('ccm:ext-install', { id: 'unity', values: {}, scope: 'user' }),
      await call('ccm:ext-install', { id: 'nope', values: {}, scope: 'user' }),
      await call('ccm:ext-install', { id: 'figma', values: {}, scope: 5, dir: project }),
      await call('ccm:ext-remove', { kind: 'mcp', name: 'my server', scope: 'user' }),
      await call('ccm:ext-remove', { kind: 'plugin', name: ['a@m', 'a@m', 'b@m'], scope: 'local', dir: project }),
      await call('ccm:ext-remove', { kind: 'plugin', name: '-x@m', scope: 'user' }),
      await call('ccm:ext-source', { type: 'marketplace', repo: 'owner/cool' }),
      await call('ccm:ext-source', { type: 'marketplace', repo: 'owner/dup' }),
      await call('ccm:ext-source', { type: 'marketplace', repo: 'bad' }),
      await call('ccm:ext-source', { type: 'mcp', name: 'remote', url: 'HTTPS://Example.COM:443/a/../b c?x=1', header: 'Authorization: Bearer tok123456', scope: 'user' }),
      await call('ccm:ext-source', { type: 'mcp', name: 'remote', url: 'ftp://x', scope: 'user' }),
      await call('ccm:ext-source', { type: 'mcp', name: 'remote', url: 'https://x', header: 'bad header', scope: 'user' }),
      await call('ccm:ext-source', { type: 'plugin', ids: ['term@m', 'ok@m'], scope: 'user' }),
      await call('ccm:ext-login', 'figma', null),
      await call('ccm:ext-login', '-bad', null),
      await call('ccm:ext-login', 'figma', 'rel'),
    ])) as unknown[]
    expect(JSON.stringify(a)).not.toContain('ghp_')
    expect(JSON.stringify(a[19])).toContain('needsTerminal')
  }, 120_000)

  it('project tools: read and toggle give the same answers and the same files', async () => {
    const claudeJson = path.join(home, '.claude.json')
    const local = path.join(project, '.claude', 'settings.local.json')
    const t = (call: (ch: string, ...args: unknown[]) => Promise<unknown>, kind: string, name: string, enabled: boolean) => call('ccm:ext-project-toggle', { dir: project, kind, name, enabled })
    const a = (await both(async (call) => {
      writeFileSync(path.join(home, '.claude', 'settings.json'), JSON.stringify({ enabledPlugins: { 'a@m': true, 'b@m': false } }))
      const out: unknown[] = [await call('ccm:ext-project', project), await call('ccm:ext-project', 'rel'), await call('ccm:ext-project', null)]
      out.push(await t(call, 'mcp', 'figma', false), await t(call, 'mcp', 'claude.ai Google Drive', false), await t(call, 'mcp', 'figma', false))
      out.push(await t(call, 'mcp', 'orrery', false), await t(call, 'mcp', '-x', false), await t(call, 'mcp', '', false), await t(call, 'x', 'a', false))
      out.push(await t(call, 'plugin', 'a@m', false), await t(call, 'plugin', 'a@m', false))
      out.push(await call('ccm:ext-project', project), await call('ccm:ext-project', project, true))
      out.push(readFileSync(claudeJson, 'utf8'), readFileSync(local, 'utf8'))
      out.push(await t(call, 'mcp', 'figma', true), await t(call, 'plugin', 'a@m', true))
      out.push(readFileSync(claudeJson, 'utf8'), readFileSync(local, 'utf8'))
      return out
    })) as unknown[]
    expect(JSON.stringify(a[13])).toContain('claude.ai Google Drive')
  }, 60_000)

  it('cancel stops before any change', async () => {
    answer = false
    await both(async (call) => [await call('ccm:ext-install', { id: 'figma', values: {}, scope: 'user' }), await call('ccm:ext-source', { type: 'marketplace', repo: 'owner/cool' })])
    answer = true
    expect(twin.logs).toEqual([])
    expect(twin.fellBack).toEqual([])
  }, 60_000)
})

describe('extensions catalog copy in core-desk', () => {
  it('matches the shared catalog', () => {
    const file = path.resolve(__dirname, '..', '..', 'core', 'crates', 'core-desk', 'src', 'extensions', 'catalog.json')
    expect(JSON.parse(readFileSync(file, 'utf8'))).toEqual(JSON.parse(JSON.stringify(CATALOG)))
  })
})
