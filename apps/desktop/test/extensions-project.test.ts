import { mkdirSync, mkdtempSync, readFileSync, realpathSync, rmSync, writeFileSync } from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { afterAll, beforeEach, describe, expect, it } from 'vitest'
import { registerExtensionsIpc, type ExtRunner } from '../src/extensions/ipc'

const tmp = realpathSync.native(mkdtempSync(path.join(os.tmpdir(), 'ccm-ext-proj-')))
const home = path.join(tmp, 'home')
const project = path.join(tmp, 'project')
const claudeJson = path.join(home, '.claude.json')
const settings = path.join(home, '.claude', 'settings.json')
const local = path.join(project, '.claude', 'settings.local.json')
const key = project.replace(/\\/g, '/')

type Handler = (event: unknown, ...args: unknown[]) => unknown

const put = (file: string, v: unknown, nl = true): string => {
  mkdirSync(path.dirname(file), { recursive: true })
  const text = JSON.stringify(v, null, 2) + (nl ? '\n' : '')
  writeFileSync(file, text)
  return text
}
const read = (file: string): string => readFileSync(file, 'utf8')
const base = (proj: unknown): unknown => ({ numStartups: 7, mcpServers: { figma: { command: 'x', env: { K: 'secret' } } }, projects: { '/other': { disabledMcpServers: ['z'], x: 1 }, [key]: proj }, tail: [1, 2, { a: null }] })

const MCP_OUT = 'plugin:github:github: https://g (HTTP) - ✔ Connected\nclaude.ai Gmail: https://m - ⚠ Needs authentication\n'

function setup(opts: { settleMs?: number; settle?: (attempt: number) => Promise<void>; run?: ExtRunner } = {}) {
  const handlers = new Map<string, Handler>()
  registerExtensionsIpc({
    ipcMain: { handle: (ch: string, fn: Handler) => handlers.set(ch, fn), removeHandler: (ch: string) => handlers.delete(ch) } as never,
    isAppFrame: () => true,
    confirm: async () => true,
    findCli: async () => '/bin/claude',
    run: opts.run ?? (async () => ({ code: 0, stdout: MCP_OUT, stderr: '' })),
    env: { PATH: '' },
    home,
    platform: process.platform,
    settleMs: opts.settleMs ?? 0,
    settle: opts.settle,
  })
  return (ch: string, ...args: unknown[]) => Promise.resolve(handlers.get(ch)!({ sender: { id: 1 } }, ...args))
}

beforeEach(() => {
  rmSync(home, { recursive: true, force: true })
  rmSync(project, { recursive: true, force: true })
  mkdirSync(project, { recursive: true })
  mkdirSync(home, { recursive: true })
})
afterAll(() => rmSync(tmp, { recursive: true, force: true }))

describe('ccm:ext-project', () => {
  it('lists names, scopes and plugin overrides without secrets', async () => {
    put(claudeJson, base({ mcpServers: { loc: { command: 'c' } }, disabledMcpServers: ['figma', 'claude.ai Google Drive'] }))
    put(settings, { enabledPlugins: { 'a@m': true, 'b@m': false, 'c@m': true } })
    put(local, { enabledPlugins: { 'c@m': false, 'd@m': true } })
    const call = setup()
    const out = (await call('ccm:ext-project', project, true)) as { ok: true; mcp: unknown[]; plugins: unknown[] }
    expect(out.mcp).toEqual([
      { name: 'figma', scope: 'user', enabled: false },
      { name: 'loc', scope: 'local', enabled: true },
      { name: 'plugin:github:github', scope: 'plugin', enabled: true },
      { name: 'claude.ai Gmail', scope: 'connector', enabled: true },
      { name: 'claude.ai Google Drive', scope: 'connector', enabled: false },
    ])
    expect(out.plugins).toEqual([
      { name: 'a@m', enabled: true, scope: 'user' },
      { name: 'c@m', enabled: false, scope: 'project' },
      { name: 'd@m', enabled: true, scope: 'project' },
    ])
    expect(JSON.stringify(out)).not.toMatch(/secret|command/)
    expect(await call('ccm:ext-project', 'rel')).toEqual({ ok: false, code: 'badArg' })
    expect(await call('ccm:ext-project')).toEqual({ ok: false, code: 'badArg' })
  })

  it('does not spawn the CLI without refresh', async () => {
    put(claudeJson, base({}))
    let spawned = 0
    const call = setup({
      run: async () => {
        spawned++
        return { code: 0, stdout: '', stderr: '' }
      },
    })
    await call('ccm:ext-project', project)
    expect(spawned).toBe(0)
  })
})

describe('ccm:ext-project-toggle', () => {
  it('mcp disable / enable round-trips the file byte for byte', async () => {
    const original = put(claudeJson, base({ history: [], allowedTools: ['a'] }))
    const call = setup()
    const t = (kind: string, name: string, enabled: boolean) => call('ccm:ext-project-toggle', { dir: project, kind, name, enabled })
    expect(await t('mcp', 'figma', false)).toEqual({ ok: true })
    const after = JSON.parse(read(claudeJson))
    expect(after.projects[key].disabledMcpServers).toEqual(['figma'])
    expect(after.projects['/other']).toEqual({ disabledMcpServers: ['z'], x: 1 })
    expect(read(claudeJson).endsWith('\n')).toBe(true)
    expect(read(`${claudeJson}.ccm-backup`)).toBe(original)
    expect(await t('mcp', 'figma', false)).toEqual({ ok: true })
    expect(await t('mcp', 'claude.ai Google Drive', false)).toEqual({ ok: true })
    expect(await t('mcp', 'figma', true)).toEqual({ ok: true })
    expect(await t('mcp', 'claude.ai Google Drive', true)).toEqual({ ok: true })
    expect(read(claudeJson)).toBe(original)
  })

  it('creates a missing project entry with forward slashes and keeps no-newline files', async () => {
    writeFileSync(claudeJson, JSON.stringify({ projects: {} }, null, 2))
    const call = setup()
    expect(await call('ccm:ext-project-toggle', { dir: project, kind: 'mcp', name: 'grapuco', enabled: false })).toEqual({ ok: true })
    expect(read(claudeJson).endsWith('\n')).toBe(false)
    expect(JSON.parse(read(claudeJson)).projects[key]).toEqual({ disabledMcpServers: ['grapuco'] })
    expect(await call('ccm:ext-project-toggle', { dir: project, kind: 'mcp', name: 'grapuco', enabled: true })).toEqual({ ok: true })
    expect(JSON.parse(read(claudeJson)).projects[key]).toEqual({})
  })

  it('matches an existing key by case and slashes on Windows', async () => {
    if (process.platform !== 'win32') return
    const odd = project.replace(/\//g, '\\').toUpperCase()
    put(claudeJson, { projects: { [odd]: { disabledMcpServers: ['a'] } } })
    const call = setup()
    const out = (await call('ccm:ext-project', project)) as { mcp: { name: string; enabled: boolean }[] }
    expect(out.mcp).toEqual([{ name: 'a', scope: 'user', enabled: false }])
    expect(await call('ccm:ext-project-toggle', { dir: project, kind: 'mcp', name: 'b', enabled: false })).toEqual({ ok: true })
    const v = JSON.parse(read(claudeJson))
    expect(Object.keys(v.projects)).toEqual([odd])
    expect(v.projects[odd].disabledMcpServers).toEqual(['a', 'b'])
  })

  it('plugin override: false in settings.local.json, enabling removes the key', async () => {
    const call = setup()
    const t = (enabled: boolean, name = 'a@m') => call('ccm:ext-project-toggle', { dir: project, kind: 'plugin', name, enabled })
    expect(await t(false)).toEqual({ ok: true })
    expect(read(local)).toBe('{\n  "enabledPlugins": {\n    "a@m": false\n  }\n}\n')
    put(local, { model: 'x', enabledPlugins: { 'a@m': false, 'b@m': false } })
    expect(await t(true)).toEqual({ ok: true })
    expect(JSON.parse(read(local))).toEqual({ model: 'x', enabledPlugins: { 'b@m': false } })
    expect(await t(true, 'b@m')).toEqual({ ok: true })
    expect(JSON.parse(read(local))).toEqual({ model: 'x' })
    writeFileSync(local, '{ nope')
    expect(await t(false)).toEqual({ ok: false, code: 'io' })
    expect(read(local)).toBe('{ nope')
  })

  it('rejects locked, bad args, a broken file; reports busy', async () => {
    const text = put(claudeJson, { projects: {} })
    const call = setup()
    const bad = (req: unknown) => call('ccm:ext-project-toggle', req)
    expect(await bad({ dir: project, kind: 'mcp', name: 'orrery', enabled: false })).toEqual({ ok: false, code: 'locked' })
    for (const req of [
      null,
      { dir: 'rel', kind: 'mcp', name: 'a', enabled: false },
      { kind: 'mcp', name: 'a', enabled: false },
      { dir: project, kind: 'x', name: 'a', enabled: false },
      { dir: project, kind: 'mcp', name: '', enabled: false },
      { dir: project, kind: 'mcp', name: '-x', enabled: false },
      { dir: project, kind: 'mcp', name: 'a\nb', enabled: false },
      { dir: project, kind: 'mcp', name: 'a'.repeat(201), enabled: false },
      { dir: project, kind: 'mcp', name: 'a', enabled: 'no' },
    ]) {
      expect(await bad(req)).toEqual({ ok: false, code: 'badArg' })
    }
    expect(read(claudeJson)).toBe(text)
    writeFileSync(claudeJson, '{ broken')
    expect(await bad({ dir: project, kind: 'mcp', name: 'a', enabled: false })).toEqual({ ok: false, code: 'io' })
    expect(read(claudeJson)).toBe('{ broken')
    rmSync(claudeJson)
    expect(await bad({ dir: project, kind: 'mcp', name: 'a', enabled: false })).toEqual({ ok: false, code: 'io' })

    put(claudeJson, { projects: {} })
    const slow = setup({ settleMs: 60 })
    const first = slow('ccm:ext-project-toggle', { dir: project, kind: 'mcp', name: 'a', enabled: false })
    await new Promise((r) => setTimeout(r, 10))
    expect(await slow('ccm:ext-project-toggle', { dir: project, kind: 'mcp', name: 'b', enabled: false })).toEqual({ ok: false, code: 'busy' })
    expect(await first).toEqual({ ok: true })
  })

  it('a foreign write that drops the change is re-applied once, then reported as conflict', async () => {
    const original = put(claudeJson, base({}))
    let settles = 0
    const once = setup({
      settle: async (attempt) => {
        settles++
        if (attempt === 0) writeFileSync(claudeJson, original)
      },
    })
    expect(await once('ccm:ext-project-toggle', { dir: project, kind: 'mcp', name: 'figma', enabled: false })).toEqual({ ok: true })
    expect(settles).toBe(2)
    expect(JSON.parse(read(claudeJson)).projects[key].disabledMcpServers).toEqual(['figma'])
    writeFileSync(claudeJson, original)
    const always = setup({ settle: async () => writeFileSync(claudeJson, original) })
    expect(await always('ccm:ext-project-toggle', { dir: project, kind: 'mcp', name: 'figma', enabled: false })).toEqual({ ok: false, code: 'conflict' })
  })
})
