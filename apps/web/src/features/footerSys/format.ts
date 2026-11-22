export const SYS_WARN_AT = 85
export const SYS_POLL_MS = 3000
export const SYS_GB = 1024 ** 3

export interface SysSample {
  cpu: number | null
  ram: number | null
  totalBytes: number
  usedBytes: number
  threads: number
}

export type SysLevel = 'ok' | 'warn'

const num = (v: unknown): number | null => (typeof v === 'number' && Number.isFinite(v) ? v : null)

export function parseSample(raw: unknown): SysSample | null {
  if (!raw || typeof raw !== 'object') return null
  const o = raw as Record<string, unknown>
  const cpu = num(o.cpu)
  const ram = num(o.ram)
  if (cpu === null && ram === null) return null
  return { cpu, ram, totalBytes: num(o.totalBytes) ?? 0, usedBytes: num(o.usedBytes) ?? 0, threads: Math.max(1, Math.round(num(o.threads) ?? 1)) }
}

export function sysPct(value: number | null): number | null {
  return value === null ? null : Math.min(100, Math.max(0, Math.round(value)))
}

export function sysLevel(pct: number | null): SysLevel {
  return pct !== null && pct >= SYS_WARN_AT ? 'warn' : 'ok'
}

export function sysChanged(prev: SysSample | null, next: SysSample): boolean {
  if (!prev) return true
  const moved = (a: number | null, b: number | null): boolean => (a === null || b === null ? a !== b : Math.abs(a - b) >= 1)
  return moved(prev.cpu, next.cpu) || moved(prev.ram, next.ram)
}

export function gbLabel(bytes: number, locale: string): string {
  return new Intl.NumberFormat(locale, { maximumFractionDigits: 1 }).format(bytes / SYS_GB)
}

type Translate = (key: string, params?: Record<string, unknown>) => string

export function sysTips(sample: SysSample, locale: string, t: Translate): { cpu: string; ram: string } {
  const cpu = sysPct(sample.cpu)
  const ram = sysPct(sample.ram)
  return {
    cpu: cpu === null ? '' : t('footerSys.cpuTip', { pct: cpu, threads: sample.threads }),
    ram: ram === null ? '' : t('footerSys.ramTip', { used: gbLabel(sample.usedBytes, locale), total: gbLabel(sample.totalBytes, locale) }),
  }
}
