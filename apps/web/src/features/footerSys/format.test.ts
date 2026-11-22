import { describe, expect, it } from 'vitest'
import { gbLabel, parseSample, sysChanged, sysLevel, sysPct, sysTips, SYS_GB, type SysSample } from './format'

const sample = (over: Partial<SysSample> = {}): SysSample => ({ cpu: 23.4, ram: 71.2, totalBytes: 64 * SYS_GB, usedBytes: 45.6 * SYS_GB, threads: 20, ...over })

const t = (key: string, p: Record<string, unknown> = {}): string =>
  key === 'footerSys.cpuTip' ? `CPU ${p.pct}%, ${p.threads} luồng` : `RAM ${p.used} / ${p.total} GB`

describe('sysPct', () => {
  it('rounds and clamps to 0..100', () => {
    expect(sysPct(23.4)).toBe(23)
    expect(sysPct(23.5)).toBe(24)
    expect(sysPct(-3)).toBe(0)
    expect(sysPct(140)).toBe(100)
    expect(sysPct(null)).toBeNull()
  })
})

describe('sysLevel', () => {
  it('warns from 85 percent', () => {
    expect(sysLevel(84)).toBe('ok')
    expect(sysLevel(85)).toBe('warn')
    expect(sysLevel(100)).toBe('warn')
    expect(sysLevel(null)).toBe('ok')
  })
})

describe('sysChanged', () => {
  it('needs a move of 1 point on cpu or ram', () => {
    expect(sysChanged(null, sample())).toBe(true)
    expect(sysChanged(sample(), sample({ cpu: 23.9, ram: 71.9 }))).toBe(false)
    expect(sysChanged(sample(), sample({ cpu: 24.4 }))).toBe(true)
    expect(sysChanged(sample(), sample({ ram: 70.1 }))).toBe(true)
    expect(sysChanged(sample(), sample({ cpu: null }))).toBe(true)
  })
})

describe('sysTips', () => {
  it('formats cpu with threads and ram with locale decimals', () => {
    expect(sysTips(sample(), 'vi', t)).toEqual({ cpu: 'CPU 23%, 20 luồng', ram: 'RAM 45,6 / 64 GB' })
    expect(sysTips(sample(), 'en', t).ram).toBe('RAM 45.6 / 64 GB')
  })

  it('leaves out a missing metric', () => {
    expect(sysTips(sample({ cpu: null }), 'en', t).cpu).toBe('')
  })
})

describe('gbLabel', () => {
  it('drops a zero decimal', () => {
    expect(gbLabel(16 * SYS_GB, 'en')).toBe('16')
  })
})

describe('parseSample', () => {
  it('accepts the core payload', () => {
    expect(parseSample({ cpu: 12.5, ram: 40, totalBytes: 100, usedBytes: 40, threads: 8 })).toEqual({ cpu: 12.5, ram: 40, totalBytes: 100, usedBytes: 40, threads: 8 })
  })

  it('rejects junk and empty samples', () => {
    expect(parseSample(null)).toBeNull()
    expect(parseSample('x')).toBeNull()
    expect(parseSample({ cpu: null, ram: null })).toBeNull()
  })

  it('keeps a sample that has only ram', () => {
    expect(parseSample({ cpu: null, ram: 50 })?.ram).toBe(50)
  })
})
