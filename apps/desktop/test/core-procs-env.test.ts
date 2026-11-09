import { describe, expect, it } from 'vitest'
import { coreProcsEnv } from '../src/coreProcsEnv'
import { coreStarts } from '../src/resources/kill'

const base = { packaged: false, resourcesPath: 'R', repo: 'D:\\repo', platform: 'win32' as const }

describe('coreProcsEnv', () => {
  it('keeps the core binary for usage and transcripts when CCM_CORE_PROCS=node', () => {
    expect(Object.keys(coreProcsEnv({ ...base, env: { CCM_CORE_PROCS: 'node' } }))).toEqual(['CCM_CORE_BIN'])
    expect(coreProcsEnv({ ...base, env: { CCM_CORE_PROCS: ' Node ' } }).CCM_CORE_PROCS).toBeUndefined()
  })

  it('passes the flag and the core binary path to the collector by default', () => {
    const env = coreProcsEnv({ ...base, env: {} })
    expect(env.CCM_CORE_PROCS).toBe('rust')
    expect(env.CCM_CORE_BIN).toMatch(/orrery-core\.exe$/)
    expect(coreProcsEnv({ ...base, env: { CCM_CORE_PROCS: 'rust', CCM_CORE_BIN: 'X:\\core.exe' } }).CCM_CORE_BIN).toBe('X:\\core.exe')
  })
})

describe('liveStarts from core rows', () => {
  it('squashes the padded day of an lstart so it matches the collector text', () => {
    const rows = [
      { pid: 10, startRaw: 'Mon Nov  9 08:05:03 2026' },
      { pid: 20, startRaw: null },
      { pid: 30, startRaw: '133000000000000000' },
    ]
    const starts = coreStarts(rows, [10, 20, 99])
    expect(starts.get(10)).toBe('Mon Nov 9 08:05:03 2026')
    expect(starts.has(20)).toBe(false)
    expect(starts.has(30)).toBe(false)
    expect(coreStarts(rows, [30]).get(30)).toBe('133000000000000000')
  })
})
