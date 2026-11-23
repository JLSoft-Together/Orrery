export interface CmdEntry {
  cmd: string
  code: number
  ms: number
  at: number
}

export const HISTORY_MAX = 200
export const HISTORY_SHOWN_MAX = 200

interface HistoryBridge {
  cmdHistList?: (project: string) => Promise<unknown>
  cmdHistAdd?: (project: string, entry: CmdEntry) => Promise<unknown>
  cmdHistRemove?: (project: string, entry: Pick<CmdEntry, 'at' | 'cmd'>) => Promise<unknown>
}

const bridge = (): HistoryBridge | undefined => (window as unknown as { ccmDesktop?: HistoryBridge }).ccmDesktop

export const hasCmdHistory = (): boolean => typeof bridge()?.cmdHistList === 'function'

function isEntry(v: unknown): v is CmdEntry {
  if (!v || typeof v !== 'object') return false
  const e = v as Record<string, unknown>
  return typeof e.cmd === 'string' && typeof e.code === 'number' && typeof e.at === 'number' && typeof e.ms === 'number'
}

export async function loadCmdHistory(project: string): Promise<CmdEntry[]> {
  try {
    const list = await bridge()?.cmdHistList?.(project)
    return Array.isArray(list) ? list.filter(isEntry).slice(0, HISTORY_MAX) : []
  } catch {
    return []
  }
}

export async function addCmdHistory(project: string, entry: CmdEntry): Promise<void> {
  try {
    await bridge()?.cmdHistAdd?.(project, entry)
  } catch {
    return
  }
}

export async function removeCmdHistory(project: string, entry: Pick<CmdEntry, 'at' | 'cmd'>): Promise<void> {
  try {
    await bridge()?.cmdHistRemove?.(project, entry)
  } catch {
  }
}
export function filterHistory(items: readonly CmdEntry[], query: string): CmdEntry[] {
  const words = query.toLowerCase().split(/\s+/).filter(Boolean)
  if (!words.length) return items.slice(0, HISTORY_SHOWN_MAX)
  return items.filter((e) => {
    const text = e.cmd.toLowerCase()
    return words.every((w) => text.includes(w))
  })
}

export function moveSelection(sel: number, delta: number, count: number): number {
  if (count <= 0) return 0
  return (((sel + delta) % count) + count) % count
}

export function isMultiline(cmd: string): boolean {
  return /[\r\n]/.test(cmd)
}

export type RunMode = 'fill' | 'run'
export function pickMode(shift: boolean, entry: CmdEntry, atPrompt: boolean): RunMode {
  return shift && !isMultiline(entry.cmd) && atPrompt ? 'run' : 'fill'
}
export function clockText(at: number, now: number): string {
  const d = new Date(at)
  const n = new Date(now)
  const pad = (v: number): string => String(v).padStart(2, '0')
  const time = `${pad(d.getHours())}:${pad(d.getMinutes())}`
  const same = d.getFullYear() === n.getFullYear() && d.getMonth() === n.getMonth() && d.getDate() === n.getDate()
  return same ? time : `${d.getDate()}/${d.getMonth() + 1} ${time}`
}
