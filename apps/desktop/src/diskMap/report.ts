import type { DiskDiffRow } from '@ccm/shared'
import { buildCompared, freeDrops, pathKey, topGrowth, type Compared, type Snapshot } from './snapshots'

export const FREE_DROP_BYTES = 5 * 1024 ** 3
const TOP_N = 5
const ALERT_GAP_MS = 6 * 3600_000

export type ReportTrigger = 'alert' | 'bgscan'
export type ReportLocale = 'vi' | 'en'

export interface ReportDrop {
  label: string
  free: number
  total: number
  drop: number
}

export interface ReportInput {
  trigger: ReportTrigger
  locale: ReportLocale
  platform: NodeJS.Platform
  home: string
  since: number
  drops: ReportDrop[]
  growth: DiskDiffRow[]
  largest: { path: string; size: number }[]
}

export function fmtBytes(n: number, locale: ReportLocale): string {
  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  let v = Math.max(0, n)
  let i = 0
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024
    i++
  }
  const s = i === 0 ? String(Math.round(v)) : v.toLocaleString(locale === 'vi' ? 'vi-VN' : 'en-US', { maximumFractionDigits: v >= 100 ? 0 : 1 })
  return `${s} ${units[i]}`
}

export function shortPath(p: string, home: string, platform: NodeJS.Platform): string {
  const sep = platform === 'win32' ? '\\' : '/'
  const cut = (s: string): string => s.replace(/[\\/]+$/, '')
  const hk = pathKey(home, platform)
  const pk = pathKey(p, platform)
  let shown = p
  if (hk && (pk === hk || pk.startsWith(hk + '\\') || pk.startsWith(hk + '/'))) {
    const rest = cut(p).slice(cut(home).length).replace(/^[\\/]+/, '')
    shown = rest ? `~${sep}${rest}` : '~'
  }
  const parts = cut(shown).split(/[\\/]/).filter((x) => x.length > 0)
  const lead = /^[\\/]/.test(shown) ? sep : ''
  if (parts.length <= 4) return lead + parts.join(sep) || shown
  return `${lead}${parts[0]}${sep}...${sep}${parts.slice(-2).join(sep)}`
}

const TEXT = {
  vi: {
    alert: 'Ổ đĩa sắp đầy',
    bgscan: 'Quét nền ổ đĩa xong',
    drive: (l: string, free: string, total: string, drop: string, since: string) => `Ổ ${l} còn trống ${free} / ${total}, giảm ${drop} so với lần quét ngày ${since}.`,
    growth: 'Phình nhiều nhất',
    largest: 'Lớn nhất',
  },
  en: {
    alert: 'Disk almost full',
    bgscan: 'Background disk scan finished',
    drive: (l: string, free: string, total: string, drop: string, since: string) => `Drive ${l} has ${free} free of ${total}, down ${drop} since the scan on ${since}.`,
    growth: 'Grew the most',
    largest: 'Largest',
  },
}

function day(at: number, locale: ReportLocale): string {
  return new Date(at).toLocaleDateString(locale === 'vi' ? 'vi-VN' : 'en-GB', { day: '2-digit', month: '2-digit', year: 'numeric' })
}

export function buildReport(i: ReportInput): string {
  const t = TEXT[i.locale]
  const lines = [i.trigger === 'alert' ? t.alert : t.bgscan]
  for (const d of i.drops) lines.push(t.drive(d.label, fmtBytes(d.free, i.locale), fmtBytes(d.total, i.locale), fmtBytes(d.drop, i.locale), day(i.since, i.locale)))
  if (i.growth.length) {
    lines.push('', `${t.growth}:`)
    i.growth.slice(0, TOP_N).forEach((r, n) => lines.push(`${n + 1}. ${shortPath(r.path, i.home, i.platform)}  +${fmtBytes(r.delta, i.locale)} (${fmtBytes(r.after, i.locale)})`))
  }
  if (i.largest.length) {
    lines.push('', `${t.largest}:`)
    i.largest.slice(0, TOP_N).forEach((r, n) => lines.push(`${n + 1}. ${shortPath(r.path, i.home, i.platform)}  ${fmtBytes(r.size, i.locale)}`))
  }
  return lines.join('\n')
}

export function largestDirs(c: Compared, platform: NodeJS.Platform, limit = TOP_N): { path: string; size: number }[] {
  const size = (k: string): number => c.nodes.get(k)?.after ?? 0
  const cands = [...c.nodes.entries()].filter(([, n]) => !n.root && (n.after ?? 0) > 0)
  const explained = (k: string): boolean => (c.children.get(k) ?? []).some((ck) => size(ck) >= size(k) * 0.8)
  const sorted = cands.filter(([k]) => !explained(k)).sort((a, b) => size(b[0]) - size(a[0]) || a[0].localeCompare(b[0]))
  const out: { key: string; path: string; size: number }[] = []
  for (const [key, n] of sorted) {
    const k = pathKey(key, platform)
    if (out.some((o) => o.key.startsWith(k + '\\') || o.key.startsWith(k + '/') || k.startsWith(o.key + '\\') || k.startsWith(o.key + '/'))) continue
    out.push({ key: k, path: n.path, size: n.after ?? 0 })
    if (out.length >= limit) break
  }
  return out.map((o) => ({ path: o.path, size: o.size }))
}

export function composeReport(base: Snapshot, cur: Snapshot, trigger: ReportTrigger, locale: ReportLocale, platform: NodeJS.Platform, home: string, minDrop = FREE_DROP_BYTES): string | null {
  const drops = freeDrops(base, cur, platform, minDrop)
  if (!drops.length) return null
  const diff = buildCompared(base, cur, platform)
  const self = buildCompared(cur, cur, platform)
  return buildReport({ trigger, locale, platform, home, since: base.at, drops, growth: topGrowth(diff, TOP_N), largest: largestDirs(self, platform, TOP_N) })
}

export interface ReportDeps {
  platform: NodeJS.Platform
  home: string
  locale: () => ReportLocale
  enabled: () => boolean
  send: (text: string) => Promise<boolean>
  now?: () => number
}

export class DiskReporter {
  private alertAt = -Infinity
  private busy = false

  constructor(private readonly deps: ReportDeps) {}

  async report(base: Snapshot | null, cur: Snapshot | null, trigger: ReportTrigger): Promise<boolean> {
    if (!base || !cur || !this.deps.enabled() || this.busy) return false
    const now = (this.deps.now ?? Date.now)()
    if (trigger === 'alert' && now - this.alertAt < ALERT_GAP_MS && now >= this.alertAt) return false
    const body = composeReport(base, cur, trigger, this.deps.locale(), this.deps.platform, this.deps.home)
    if (!body) return false
    this.busy = true
    try {
      const ok = await this.deps.send(body)
      if (ok && trigger === 'alert') this.alertAt = now
      return ok
    } catch {
      return false
    } finally {
      this.busy = false
    }
  }
}


