import { describe, expect, it } from 'vitest'
import { agoOf, lastActive, monogram, projectTone, PROJECT_TONES, runCounts } from './projectRow'

const row = (state: string, since: number) => ({ state, since })
const cluster = (rows: ReturnType<typeof row>[]) => ({ checkouts: [{ rows }] }) as never

describe('projectRow', () => {
  it('tone is stable and in range', () => {
    expect(projectTone('repo:/a')).toBe(projectTone('repo:/a'))
    for (const k of ['a', 'bb', 'home', 'plain:/x/y']) expect(projectTone(k)).toBeLessThan(PROJECT_TONES)
  })

  it('monogram is the first letter, upper case', () => {
    expect(monogram('orrery')).toBe('O')
    expect(monogram('  -deck')).toBe('D')
    expect(monogram('~')).toBe('?')
  })

  it('counts running and waiting rows', () => {
    expect(runCounts(cluster([row('working', 1), row('subagents', 1), row('asked', 1), row('idle', 1)]))).toEqual({ running: 2, waiting: 1 })
  })

  it('last active is the newest row time or null', () => {
    expect(lastActive(cluster([row('idle', 5), row('idle', 9)]))).toBe(9)
    expect(lastActive(cluster([]))).toBeNull()
  })

  it('ago buckets', () => {
    const now = 10 * 86400_000
    expect(agoOf(now - 5_000, now)).toEqual({ key: 'now' })
    expect(agoOf(now - 5 * 60_000, now)).toEqual({ key: 'minutes', n: 5 })
    expect(agoOf(now - 3 * 3600_000, now)).toEqual({ key: 'hours', n: 3 })
    expect(agoOf(now - 86400_000, now)).toEqual({ key: 'yesterday' })
    expect(agoOf(now - 3 * 86400_000, now)).toEqual({ key: 'days', n: 3 })
  })
})
