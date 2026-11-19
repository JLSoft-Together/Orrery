import { RAM_KILL_MAX, type RamKillTarget, type RamLock, type RamProc, type RamSnapshot, type RamTag } from '@ccm/shared'

export interface RawProc {
  pid: number
  ppid: number
  name: string
  start: string | null
  startMs: number | null
  ramBytes: number | null
  cpuMs: number | null
}

export interface RamSamplerDeps {
  read: () => Promise<RawProc[] | null>
  selfPid: number
  platform: NodeJS.Platform
  cpus: () => number
  totalMem: () => number
  freeMem: () => number
  now?: () => number
}

const AGENT = new Set(['claude', 'codex', 'codewhale', 'deepseek', 'gemini', 'aider', 'cursor-agent'])
const DEV = new Set([
  'node', 'java', 'javaw', 'gradle', 'gradlew', 'kotlin-daemon', 'adb', 'emulator', 'docker', 'dockerd', 'com.docker.backend', 'com.docker.build', 'docker desktop',
  'vmmemwsl', 'vmmem', 'dart', 'dartaotruntime', 'flutter', 'python', 'python3', 'pythonw', 'cargo', 'rustc', 'rust-analyzer', 'go', 'gopls', 'deno', 'bun',
  'code', 'studio64', 'studio', 'idea64', 'idea', 'xcode', 'simulator', 'tsserver', 'esbuild', 'vite', 'npm', 'pnpm', 'yarn', 'git', 'qemu-system',
])
const SYSTEM_WIN = new Set([
  'system', 'registry', 'smss', 'csrss', 'wininit', 'winlogon', 'services', 'lsass', 'lsaiso', 'svchost', 'fontdrvhost', 'dwm', 'memory compression', 'secure system',
  'system idle process', 'idle', 'explorer', 'sihost', 'ctfmon', 'audiodg', 'spoolsv', 'searchindexer', 'msmpeng', 'securityhealthservice', 'wudfhost', 'taskhostw',
])
const SYSTEM_MAC = new Set(['kernel_task', 'launchd', 'windowserver', 'loginwindow', 'logd', 'opendirectoryd', 'coreaudiod', 'systemstats', 'securityd', 'configd', 'mds', 'mds_stores', 'fseventsd', 'dock', 'finder', 'systemuiserver'])

export function baseName(name: string): string {
  return name.trim().toLowerCase().replace(/\.exe$/, '')
}

export function tagOf(name: string): RamTag | null {
  const b = baseName(name)
  if (AGENT.has(b)) return 'agent'
  if (DEV.has(b) || b.startsWith('qemu-system') || b.startsWith('com.docker.')) return 'dev'
  return null
}

export function systemName(name: string, platform: NodeJS.Platform): boolean {
  const b = baseName(name)
  return platform === 'darwin' ? SYSTEM_MAC.has(b) : platform === 'win32' ? SYSTEM_WIN.has(b) : b === 'init' || b === 'systemd'
}

export const sameStart = (a: string | null | undefined, b: string | null | undefined): boolean => !!a && !!b && a.trim().replace(/\s+/g, ' ') === b.trim().replace(/\s+/g, ' ')

export function selfTree(rows: readonly RawProc[], selfPid: number): { self: Set<number>; parents: Set<number> } {
  const kids = new Map<number, number[]>()
  const byPid = new Map<number, RawProc>()
  for (const r of rows) {
    byPid.set(r.pid, r)
    if (r.pid === r.ppid) continue
    const list = kids.get(r.ppid) ?? []
    list.push(r.pid)
    kids.set(r.ppid, list)
  }
  const self = new Set<number>([selfPid])
  const stack = [selfPid]
  while (stack.length) {
    for (const k of kids.get(stack.pop()!) ?? []) {
      if (self.has(k)) continue
      self.add(k)
      stack.push(k)
    }
  }
  const parents = new Set<number>()
  let cur = byPid.get(selfPid)
  while (cur && cur.ppid > 0 && !self.has(cur.ppid) && !parents.has(cur.ppid)) {
    parents.add(cur.ppid)
    cur = byPid.get(cur.ppid)
  }
  return { self, parents }
}

export function lockOf(row: RawProc, tree: { self: Set<number>; parents: Set<number> }, platform: NodeJS.Platform): RamLock | null {
  if (tree.self.has(row.pid)) return 'self'
  if (tree.parents.has(row.pid)) return 'parent'
  if (row.pid <= 4 || (platform !== 'win32' && row.pid <= 1) || systemName(row.name, platform)) return 'system'
  return null
}

export function parseRamTargets(raw: unknown): RamKillTarget[] | null {
  const list = (raw as { targets?: unknown } | null)?.targets
  if (!Array.isArray(list) || list.length < 1 || list.length > RAM_KILL_MAX) return null
  const out: RamKillTarget[] = []
  const seen = new Set<number>()
  for (const t of list) {
    const o = typeof t === 'object' && t !== null ? (t as Record<string, unknown>) : null
    if (!o || typeof o.pid !== 'number' || !Number.isInteger(o.pid) || o.pid <= 0 || o.pid > 0x7fffffff) return null
    if (typeof o.start !== 'string' || !o.start.trim() || o.start.length > 64) return null
    if (seen.has(o.pid)) continue
    seen.add(o.pid)
    out.push({ pid: o.pid, start: o.start })
  }
  return out
}

export class RamSampler {
  private prev = new Map<string, { cpuMs: number; at: number }>()
  private rows = new Map<number, RamProc>()
  private reading: Promise<RamSnapshot> | null = null
  private lastSnap: RamSnapshot | null = null

  constructor(private readonly deps: RamSamplerDeps) {}

  get(pid: number): RamProc | undefined {
    return this.rows.get(pid)
  }

  snapshot(): Promise<RamSnapshot> {
    const now = this.deps.now?.() ?? Date.now()
    if (this.lastSnap && now - this.lastSnap.at < 900) return Promise.resolve(this.lastSnap)
    this.reading ??= this.read().finally(() => {
      this.reading = null
    })
    return this.reading
  }

  private async read(): Promise<RamSnapshot> {
    const raw = await this.deps.read().catch(() => null)
    const at = this.deps.now?.() ?? Date.now()
    const cpus = Math.max(1, this.deps.cpus())
    const base = { at, totalMem: this.deps.totalMem(), freeMem: this.deps.freeMem(), cpus }
    if (!raw) return { ...base, ok: false, procs: [] }
    const tree = selfTree(raw, this.deps.selfPid)
    const next = new Map<string, { cpuMs: number; at: number }>()
    const rows = new Map<number, RamProc>()
    const procs: RamProc[] = []
    for (const r of raw) {
      const key = `${r.pid}:${r.start ?? ''}`
      let cpu: number | null = null
      if (r.cpuMs !== null && r.start) {
        const p = this.prev.get(key)
        if (p && at > p.at) cpu = Math.max(0, Math.min(100, ((r.cpuMs - p.cpuMs) / ((at - p.at) * cpus)) * 100))
        next.set(key, { cpuMs: r.cpuMs, at })
      }
      const lock = r.start ? lockOf(r, tree, this.deps.platform) : (lockOf(r, tree, this.deps.platform) ?? 'system')
      const proc: RamProc = { pid: r.pid, ppid: r.ppid, start: r.start ?? '', name: r.name, ramBytes: r.ramBytes, cpu, tag: lock === 'self' ? 'self' : tagOf(r.name), lock }
      rows.set(r.pid, proc)
      procs.push(proc)
    }
    this.prev = next
    this.rows = rows
    procs.sort((a, b) => (b.ramBytes ?? -1) - (a.ramBytes ?? -1))
    this.lastSnap = { ...base, ok: true, procs }
    return this.lastSnap
  }
}
