import { chmodSync, copyFileSync, existsSync, readFileSync, renameSync, rmSync, mkdirSync, writeFileSync } from 'node:fs'
import path from 'node:path'
import type { BridgeInstall } from './bridge'
import { formatSettings } from './preview/permRules'

export const BRIDGE_EXE = process.platform === 'win32' ? 'orrery-bridge.exe' : 'orrery-bridge'
export const BRIDGE_NAMES = ['statusline-bridge', 'hook-bridge'] as const
export type BridgeName = (typeof BRIDGE_NAMES)[number]

const slashes = (p: string): string => p.replaceAll('\\', '/')

export function rustBridgeSource(coreBin: string, env: NodeJS.ProcessEnv = process.env): string {
  const override = env.CCM_BRIDGE_BIN?.trim()
  return override || path.join(path.dirname(coreBin), BRIDGE_EXE)
}

export function installRustBridge(source: string, dataDir: string): string {
  const dir = path.join(dataDir, 'bin')
  mkdirSync(dir, { recursive: true })
  const target = path.join(dir, BRIDGE_EXE)
  const want = readFileSync(source)
  if (existsSync(target) && readFileSync(target).equals(want)) return target
  const tmp = `${target}.${process.pid}.tmp`
  try {
    writeFileSync(tmp, want)
    if (process.platform !== 'win32') chmodSync(tmp, 0o755)
    renameSync(tmp, target)
  } catch (err) {
    rmSync(tmp, { force: true })
    if (!existsSync(target)) throw err
  }
  return target
}

export function rustBridgeCommand(exe: string, name: BridgeName): BridgeInstall {
  return { script: exe, command: `"${slashes(exe)}" ${name}` }
}

const fold = (s: string): string => (process.platform === 'win32' ? slashes(s).toLowerCase() : slashes(s))

export function oldBridgePrefixes(dataDir: string, name: BridgeName): string[] {
  const bin = slashes(path.join(dataDir, 'bin'))
  return [`node "${bin}/${name}.mjs"`, `"${bin}/${name}.cmd"`, `"${bin}/${name}.sh"`]
}

export function migratedCommand(command: string, dataDir: string, name: BridgeName, next: string): string | null {
  for (const p of oldBridgePrefixes(dataDir, name)) {
    if (command.length < p.length || fold(command.slice(0, p.length)) !== fold(p)) continue
    const rest = command.slice(p.length)
    if (rest === '' || rest.startsWith(' ')) return `${next}${rest}`
  }
  return null
}

export interface MigrateResult {
  result: 'ok' | 'none' | 'unreadable' | 'failed'
  statusLine: boolean
  hooks: number
}

const isObj = (v: unknown): v is Record<string, unknown> => !!v && typeof v === 'object' && !Array.isArray(v)

export function migrateBridges(file: string, dataDir: string, next: { statusline?: string | null; hook?: string | null }): MigrateResult {
  const none: MigrateResult = { result: 'none', statusLine: false, hooks: 0 }
  let text: string
  try {
    text = readFileSync(file, 'utf8')
  } catch {
    return none
  }
  let settings: unknown
  try {
    settings = text.trim() ? JSON.parse(text) : {}
  } catch {
    return { ...none, result: 'unreadable' }
  }
  if (!isObj(settings)) return { ...none, result: 'unreadable' }
  let line = false
  let hooks = 0
  const sl = settings.statusLine
  if (next.statusline && isObj(sl) && typeof sl.command === 'string') {
    const nc = migratedCommand(sl.command, dataDir, 'statusline-bridge', next.statusline)
    if (nc !== null) {
      sl.command = nc
      line = true
    }
  }
  if (next.hook && isObj(settings.hooks)) {
    for (const groups of Object.values(settings.hooks)) {
      if (!Array.isArray(groups)) continue
      for (const g of groups) {
        if (!isObj(g) || !Array.isArray(g.hooks)) continue
        for (const h of g.hooks) {
          if (!isObj(h) || typeof h.command !== 'string') continue
          const nc = migratedCommand(h.command, dataDir, 'hook-bridge', next.hook)
          if (nc === null) continue
          h.command = nc
          hooks++
        }
      }
    }
  }
  if (!line && !hooks) return none
  const tmp = `${file}.ccm-tmp`
  try {
    copyFileSync(file, `${file}.ccm-backup`)
    writeFileSync(tmp, formatSettings(settings, text))
    renameSync(tmp, file)
    return { result: 'ok', statusLine: line, hooks }
  } catch {
    rmSync(tmp, { force: true })
    return { result: 'failed', statusLine: line, hooks }
  }
}
