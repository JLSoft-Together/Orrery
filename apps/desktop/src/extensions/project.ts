import { chmodSync, copyFileSync, existsSync, mkdirSync, readFileSync, renameSync, rmSync, statSync, writeFileSync } from 'node:fs'
import path from 'node:path'
import type { ExtProject, ExtProjectChange, ExtProjectErrorCode, ExtProjectToggle, ProjectMcpRow, ProjectMcpScope, ProjectPluginRow } from '@ccm/shared'
import { readConfigured, sameDir } from './status'

const LOCKED = 'orrery'
const NAME_MAX = 200

type Obj = Record<string, unknown>

type Loaded = { state: 'missing' } | { state: 'bad' } | { state: 'ok'; root: Obj; newline: boolean }

const isObj = (v: unknown): v is Obj => typeof v === 'object' && v !== null && !Array.isArray(v)

function load(file: string): Loaded {
  let text: string
  try {
    text = readFileSync(file, 'utf8')
  } catch (err) {
    return (err as NodeJS.ErrnoException).code === 'ENOENT' ? { state: 'missing' } : { state: 'bad' }
  }
  try {
    const root: unknown = JSON.parse(text)
    return isObj(root) ? { state: 'ok', root, newline: text.endsWith('\n') } : { state: 'bad' }
  } catch {
    return { state: 'bad' }
  }
}

const readObj = (file: string): Obj => {
  const l = load(file)
  return l.state === 'ok' ? l.root : {}
}

const render = (root: Obj, newline: boolean): string => JSON.stringify(root, null, 2) + (newline ? '\n' : '')

function writeAtomic(file: string, text: string, backup: boolean): boolean {
  const tmp = `${file}.ccm-tmp`
  try {
    mkdirSync(path.dirname(file), { recursive: true })
    if (backup) copyFileSync(file, `${file}.ccm-backup`)
    const mode = existsSync(file) ? statSync(file).mode & 0o777 : null
    writeFileSync(tmp, text, mode === null ? undefined : { mode })
    if (mode !== null && process.platform !== 'win32') chmodSync(tmp, mode)
    renameSync(tmp, file)
    return true
  } catch {
    try {
      rmSync(tmp, { force: true })
    } catch {
      void 0
    }
    return false
  }
}

const projectKeys = (root: Obj, dir: string, platform: NodeJS.Platform): string[] =>
  isObj(root.projects) ? Object.keys(root.projects).filter((k) => sameDir(k, dir, platform)) : []

function disabledNames(root: Obj, dir: string, platform: NodeJS.Platform): string[] {
  const out: string[] = []
  const projects = root.projects
  if (!isObj(projects)) return out
  for (const key of projectKeys(root, dir, platform)) {
    const p = projects[key]
    const list = isObj(p) ? p.disabledMcpServers : undefined
    if (Array.isArray(list)) for (const n of list) if (typeof n === 'string' && !out.includes(n)) out.push(n)
  }
  return out
}

const pluginMap = (root: Obj): [string, boolean][] => (isObj(root.enabledPlugins) ? Object.entries(root.enabledPlugins).filter((e): e is [string, boolean] => typeof e[1] === 'boolean') : [])

function scopeOf(name: string, configured: { name: string; scopes: string[] }[]): ProjectMcpScope {
  const hit = configured.find((c) => c.name === name)
  if (hit) return hit.scopes.includes('local') ? 'local' : hit.scopes.includes('project') ? 'project' : 'user'
  if (name.startsWith('plugin:')) return 'plugin'
  if (name.startsWith('claude.ai ')) return 'connector'
  return 'user'
}

export const validName = (name: unknown): name is string =>
  typeof name === 'string' && name.length > 0 && [...name].length <= NAME_MAX && !name.startsWith('-') && !/[\p{Cc}]/u.test(name)

function editMcp(root: Obj, dir: string, name: string, enabled: boolean, platform: NodeJS.Platform): boolean {
  const keys = projectKeys(root, dir, platform)
  if (enabled) {
    const projects = root.projects
    if (!isObj(projects)) return false
    let changed = false
    for (const key of keys) {
      const p = projects[key]
      if (!isObj(p) || !('disabledMcpServers' in p)) continue
      const list = p.disabledMcpServers
      if (!Array.isArray(list)) throw new Error('shape')
      const next = list.filter((v) => v !== name)
      if (next.length !== list.length) {
        changed = true
        if (next.length === 0) delete p.disabledMcpServers
        else p.disabledMcpServers = next
      }
    }
    return changed
  }
  if (!('projects' in root)) root.projects = {}
  const projects = root.projects
  if (!isObj(projects)) throw new Error('shape')
  for (const key of keys) {
    const p = projects[key]
    if (!isObj(p)) continue
    const list = p.disabledMcpServers
    if (Array.isArray(list)) {
      if (list.includes(name)) return false
    } else if (list !== undefined) throw new Error('shape')
  }
  const target = keys[0] ?? dir.replace(/\\/g, '/')
  if (!(target in projects)) projects[target] = {}
  const slot = projects[target]
  if (!isObj(slot)) throw new Error('shape')
  if (!('disabledMcpServers' in slot)) slot.disabledMcpServers = []
  const list = slot.disabledMcpServers
  if (!Array.isArray(list)) throw new Error('shape')
  list.push(name)
  return true
}

export interface ProjectPaths {
  claudeJson: string
  settingsFile: string
  platform: NodeJS.Platform
}

export function readProject(paths: ProjectPaths, dir: string, cachedNames: string[]): ExtProject {
  const { platform } = paths
  const configured = readConfigured(paths.claudeJson, dir, platform)
  const disabled = disabledNames(readObj(paths.claudeJson), dir, platform)
  const names = configured.map((c) => c.name)
  for (const n of [...cachedNames, ...disabled]) if (!names.includes(n)) names.push(n)
  const mcp: ProjectMcpRow[] = names.map((n) => ({
    name: n,
    scope: scopeOf(n, configured),
    enabled: !disabled.includes(n),
    ...(n === LOCKED ? { locked: true as const } : {}),
  }))
  const user = pluginMap(readObj(paths.settingsFile))
  const local = pluginMap(readObj(path.join(dir, '.claude', 'settings.local.json')))
  const plugins: ProjectPluginRow[] = []
  for (const [name, on] of user) {
    const over = local.find(([n]) => n === name)?.[1]
    if (on || over !== undefined) plugins.push({ name, enabled: over ?? on, scope: over !== undefined ? 'project' : 'user' })
  }
  for (const [name, on] of local) if (!user.some(([n]) => n === name)) plugins.push({ name, enabled: on, scope: 'project' })
  return { ok: true, dir, mcp, plugins }
}

const fail = (code: ExtProjectErrorCode): ExtProjectToggle => ({ ok: false, code })

export async function toggleMcp(paths: ProjectPaths, dir: string, name: string, enabled: boolean, settle: (attempt: number) => Promise<void>): Promise<ExtProjectToggle> {
  const file = paths.claudeJson
  for (let attempt = 0; attempt < 2; attempt++) {
    const l = load(file)
    if (l.state !== 'ok') return fail('io')
    try {
      if (editMcp(l.root, dir, name, enabled, paths.platform) && !writeAtomic(file, render(l.root, l.newline), true)) return fail('io')
    } catch {
      return fail('io')
    }
    await settle(attempt)
    const again = load(file)
    if (again.state === 'ok' && disabledNames(again.root, dir, paths.platform).includes(name) !== enabled) return { ok: true }
  }
  return fail('conflict')
}

function editPlugin(root: Obj, name: string, enabled: boolean): boolean {
  if (enabled) {
    if (!('enabledPlugins' in root)) return false
    const ep = root.enabledPlugins
    if (!isObj(ep)) throw new Error('shape')
    if (!(name in ep)) return false
    delete ep[name]
    if (Object.keys(ep).length === 0) delete root.enabledPlugins
    return true
  }
  if (!('enabledPlugins' in root)) root.enabledPlugins = {}
  const ep = root.enabledPlugins
  if (!isObj(ep)) throw new Error('shape')
  if (ep[name] === false) return false
  ep[name] = false
  return true
}

function writePlugins(dir: string, changes: readonly ExtProjectChange[]): ExtProjectToggle {
  const file = path.join(dir, '.claude', 'settings.local.json')
  const l = load(file)
  if (l.state === 'bad') return fail('io')
  const root: Obj = l.state === 'ok' ? l.root : {}
  const newline = l.state === 'ok' ? l.newline : true
  let changed = false
  try {
    for (const c of changes) changed = editPlugin(root, c.name, c.enabled) || changed
  } catch {
    return fail('io')
  }
  return !changed || writeAtomic(file, render(root, newline), existsSync(file)) ? { ok: true } : fail('io')
}

export const togglePlugin = (dir: string, name: string, enabled: boolean): ExtProjectToggle => writePlugins(dir, [{ kind: 'plugin', name, enabled }])

export const CHANGES_MAX = 500

export function parseChanges(raw: unknown): ExtProjectChange[] | null {
  if (!Array.isArray(raw) || raw.length === 0 || raw.length > CHANGES_MAX) return null
  const out: ExtProjectChange[] = []
  for (const item of raw) {
    if (!isObj(item) || (item.kind !== 'mcp' && item.kind !== 'plugin') || !validName(item.name) || typeof item.enabled !== 'boolean') return null
    out.push({ kind: item.kind, name: item.name, enabled: item.enabled })
  }
  return out
}

export async function applyChanges(paths: ProjectPaths, dir: string, changes: readonly ExtProjectChange[], settle: (attempt: number) => Promise<void>): Promise<ExtProjectToggle> {
  const plugins = changes.filter((c) => c.kind === 'plugin')
  const mcp = changes.filter((c) => c.kind === 'mcp')
  if (plugins.length) {
    const res = writePlugins(dir, plugins)
    if (!res.ok) return res
  }
  if (!mcp.length) return { ok: true }
  const wanted = new Map(mcp.map((c) => [c.name, c.enabled]))
  const file = paths.claudeJson
  for (let attempt = 0; attempt < 2; attempt++) {
    const l = load(file)
    if (l.state !== 'ok') return fail('io')
    let changed = false
    try {
      for (const c of mcp) changed = editMcp(l.root, dir, c.name, c.enabled, paths.platform) || changed
    } catch {
      return fail('io')
    }
    if (changed) {
      if (!writeAtomic(file, render(l.root, l.newline), true)) return fail('io')
      await settle(attempt)
    }
    const again = load(file)
    if (again.state === 'ok') {
      const off = disabledNames(again.root, dir, paths.platform)
      if ([...wanted].every(([name, enabled]) => off.includes(name) !== enabled)) return { ok: true }
    }
  }
  return fail('conflict')
}
