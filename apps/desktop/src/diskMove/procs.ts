import fs from 'node:fs/promises'
import path from 'node:path'
import { commandLinesWin, coreRows } from '../../../collector/src/procSource'
import { parseProcLines, type ProcInfo } from '../diskClean/sdk'
import type { Suggestion, VolumeInfo } from './service'
import { TOOL } from './service'

export async function toolProcs(): Promise<ProcInfo[]> {
  const rows = await coreRows().catch(() => null)
  if (!rows) return []
  const tools = rows.filter((r) => TOOL.test(r.name))
  if (!tools.length) return []
  const cmds = await commandLinesWin(tools.map((r) => r.pid)).catch(() => new Map<number, string>())
  return tools.map((r) => ({ name: r.name.toLowerCase(), cmd: cmds.get(r.pid) ?? '' }))
}

export interface MoveSpotsInput {
  platform: NodeJS.Platform
  home: string
  env: NodeJS.ProcessEnv
  sdkRoot: string | null
  avdHome: string
}

export function moveSuggestions({ platform, home, env, sdkRoot, avdHome }: MoveSpotsInput): Suggestion[] {
  const lib = platform === 'win32' ? path.win32 : path.posix
  const local = env.LOCALAPPDATA || lib.join(home, 'AppData', 'Local')
  const out: Suggestion[] = [
    { kind: 'gradle', path: env.GRADLE_USER_HOME || lib.join(home, '.gradle') },
    { kind: 'pub', path: env.PUB_CACHE || (platform === 'win32' ? lib.join(local, 'Pub', 'Cache') : lib.join(home, '.pub-cache')) },
    { kind: 'avd', path: avdHome },
    ...(sdkRoot ? [{ kind: 'sdk' as const, path: sdkRoot }] : []),
    { kind: 'npm', path: env.npm_config_cache || (platform === 'win32' ? lib.join(local, 'npm-cache') : lib.join(home, '.npm')) },
    { kind: 'docker', path: platform === 'win32' ? lib.join(local, 'Docker', 'wsl') : platform === 'darwin' ? lib.join(home, 'Library', 'Containers', 'com.docker.docker') : '/var/lib/docker', guide: true },
  ]
  return out
}

export function fakeMoveSuggestions(fake: string): Suggestion[] {
  const h = path.join(fake, 'home')
  return [
    { kind: 'gradle', path: path.join(h, '.gradle') },
    { kind: 'avd', path: path.join(h, '.android', 'avd') },
    { kind: 'npm', path: path.join(h, 'npm-cache') },
    { kind: 'docker', path: path.join(h, 'Docker'), guide: true },
  ]
}

export async function fakeVolumes(fake: string): Promise<VolumeInfo[]> {
  const base = path.join(fake, 'vols')
  const out: VolumeInfo[] = []
  for (const n of (await fs.readdir(base).catch(() => [] as string[])).sort()) {
    const mount = path.join(base, n)
    const st = await fs.statfs(mount).catch(() => null)
    out.push({ mount, label: n, network: n.startsWith('net'), total: st ? Number(st.blocks) * Number(st.bsize) : 0, free: st ? Number(st.bavail) * Number(st.bsize) : 0 })
  }
  return out
}

export function fakeDev(fake: string): (p: string) => Promise<number | null> {
  const base = path.join(fake, 'vols').toLowerCase()
  return async (p) => {
    const st = await fs.lstat(p).catch(() => null)
    if (!st) return null
    const rel = path.relative(base, p.toLowerCase())
    if (rel.startsWith('..') || path.isAbsolute(rel) || !rel) return 1
    const first = rel.split(/[\/]/)[0]!
    let h = 7
    for (const c of first) h = (h * 31 + c.charCodeAt(0)) >>> 0
    return 100 + (h % 100000)
  }
}

export async function fakeProcs(fake: string): Promise<ProcInfo[]> {
  return parseProcLines(await fs.readFile(path.join(fake, 'procs.txt'), 'utf8').catch(() => ''))
}

export async function fakePick(fake: string): Promise<string | null> {
  return (await fs.readFile(path.join(fake, 'pick.txt'), 'utf8').catch(() => '')).trim() || null
}
