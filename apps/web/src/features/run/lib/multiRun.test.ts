import { describe, expect, it } from 'vitest'
import { MULTI_MAX, pickDevices, pool, runMany, variantOf, type BuildInfo, type MultiDeps, type MultiStage } from './multiRun'

const INFO: BuildInfo = { apk: 'C:/p/app.apk', appId: 'dev.x', activity: 'dev.x.Main' }
const PNG = { ok: true as const, png: 'data:image/png;base64,AA', width: 10, height: 20 }

function fakeDeps(over: Partial<MultiDeps> = {}) {
  const calls: string[] = []
  let live = 0
  let peak = 0
  const deps: MultiDeps = {
    build: async (id) => (calls.push(`build ${id}`), { ok: true, ...INFO }),
    install: async (id) => {
      calls.push(`install ${id}`)
      live++
      peak = Math.max(peak, live)
      await Promise.resolve()
      await Promise.resolve()
      live--
      return { ok: true }
    },
    open: async (id) => (calls.push(`open ${id}`), { ok: true }),
    apply: async (id, v) => (calls.push(`apply ${id} ${v.id}`), { ok: true }),
    restore: async (id) => (calls.push(`restore ${id}`), { ok: true }),
    capture: async (id) => (calls.push(`capture ${id}`), PNG),
    wait: async () => {},
    ...over,
  }
  return { deps, calls, peak: () => peak }
}

const run = (deps: MultiDeps, devices: string[], cancelled: () => boolean = () => false, onStage?: (id: string, s: MultiStage) => void) =>
  runMany({ devices, variant: variantOf('dark'), deps, isCancelled: cancelled, onStage })

describe('pool and device pick', () => {
  it('keeps at most 3 distinct devices', () => {
    expect(pickDevices(['a', 'b', 'a', 'c', 'd', 'e'])).toEqual(['a', 'b', 'c'])
    expect(MULTI_MAX).toBe(3)
  })

  it('never runs more than the limit at once and runs everything', async () => {
    let live = 0
    let peak = 0
    const done: number[] = []
    await pool([1, 2, 3, 4, 5, 6, 7], 3, async (n) => {
      live++
      peak = Math.max(peak, live)
      await Promise.resolve()
      await Promise.resolve()
      live--
      done.push(n)
    })
    expect(peak).toBe(3)
    expect(done.sort()).toEqual([1, 2, 3, 4, 5, 6, 7])
  })
})

describe('runMany', () => {
  it('builds once on the first device and installs and opens on the others', async () => {
    const { deps, calls } = fakeDeps()
    const out = await run(deps, ['a', 'b', 'c'])
    expect(calls.filter((c) => c.startsWith('build'))).toEqual(['build a'])
    expect(calls.filter((c) => c.startsWith('install')).sort()).toEqual(['install b', 'install c'])
    expect(calls.filter((c) => c.startsWith('open')).sort()).toEqual(['open b', 'open c'])
    expect(out.shots.map((s) => [s.deviceId, s.stage, s.variant])).toEqual([['a', 'done', 'dark'], ['b', 'done', 'dark'], ['c', 'done', 'dark']])
    expect(out.shots[0]).toMatchObject({ png: PNG.png, width: 10, height: 20 })
    expect(out.restoreFailed).toEqual([])
  })

  it('installs and applies the variant before opening on the other devices, restoring after the shot', async () => {
    const { deps, calls } = fakeDeps()
    await run(deps, ['a', 'b'])
    const b = calls.filter((c) => c.endsWith(' b') || c.includes(' b '))
    expect(b).toEqual(['install b', 'apply b dark', 'open b', 'capture b', 'restore b'])
  })

  it('a failure on one device does not stop the others and still restores that device', async () => {
    const { deps, calls } = fakeDeps({ install: async (id) => (id === 'b' ? { ok: false, error: 'signature' } : { ok: true }) })
    const out = await run(deps, ['a', 'b', 'c'])
    expect(out.shots.map((s) => s.stage)).toEqual(['done', 'failed', 'done'])
    expect(out.shots[1]!.error).toBe('signature')
    expect(calls).not.toContain('apply b dark')
    expect(calls).toContain('capture c')
  })

  it('restores when the capture fails or the variant fails halfway', async () => {
    const { deps, calls } = fakeDeps({
      capture: async (id) => (id === 'a' ? { ok: false, error: 'secure' } : PNG),
      apply: async (id) => (id === 'c' ? { ok: false, error: 'apply-failed' } : { ok: true }),
    })
    const out = await run(deps, ['a', 'b', 'c'])
    expect(out.shots.map((s) => s.stage)).toEqual(['failed', 'done', 'failed'])
    expect(calls).toContain('restore a')
    expect(calls).toContain('restore c')
    expect(calls).toContain('restore b')
  })

  it('restores when a step throws', async () => {
    const { deps, calls } = fakeDeps({
      open: async () => {
        throw new Error('boom')
      },
    })
    const out = await run(deps, ['a', 'b'])
    expect(out.shots[1]).toMatchObject({ stage: 'failed', error: 'boom' })
    expect(calls).toContain('restore b')
  })

  it('reports devices whose restore failed', async () => {
    const { deps } = fakeDeps({ restore: async (id) => (id === 'b' ? { ok: false, error: 'restore-failed' } : { ok: true }) })
    const out = await run(deps, ['a', 'b'])
    expect(out.restoreFailed).toEqual(['b'])
    expect(out.shots.map((s) => s.stage)).toEqual(['done', 'done'])
  })

  it('a failed build fails everything and touches no device setting', async () => {
    const { deps, calls } = fakeDeps({ build: async () => ({ ok: false, error: 'build' }) })
    const out = await run(deps, ['a', 'b'])
    expect(out.shots.map((s) => [s.stage, s.error])).toEqual([['failed', 'build'], ['failed', 'build']])
    expect(calls.some((c) => c.startsWith('apply') || c.startsWith('install'))).toBe(false)
  })

  it('a first device that failed only at install still lets the built apk go to the others', async () => {
    const { deps, calls } = fakeDeps({ build: async () => ({ ok: false, error: 'install', built: INFO }) })
    const out = await run(deps, ['a', 'b'])
    expect(out.shots.map((s) => s.stage)).toEqual(['failed', 'done'])
    expect(calls).toContain('install b')
    expect(calls).not.toContain('apply a dark')
  })

  it('cancel during the build marks every device cancelled and changes nothing on them', async () => {
    let stop = false
    const { deps, calls } = fakeDeps({
      build: async () => {
        stop = true
        return { ok: false, error: 'cancelled' }
      },
    })
    const out = await run(deps, ['a', 'b', 'c'], () => stop)
    expect(out.shots.map((s) => s.stage)).toEqual(['cancelled', 'cancelled', 'cancelled'])
    expect(calls).toEqual([])
  })

  it('cancel after the apply still restores and does not capture', async () => {
    let stop = false
    const { deps, calls } = fakeDeps({
      apply: async (id) => {
        if (id === 'b') stop = true
        return { ok: true }
      },
    })
    const out = await run(deps, ['a', 'b'], () => stop)
    expect(out.shots.find((s) => s.deviceId === 'b')!.stage).toBe('cancelled')
    expect(calls).not.toContain('capture b')
    expect(calls).not.toContain('open b')
    expect(calls).toContain('restore b')
  })

  it('runs at most 3 devices in parallel', async () => {
    const { deps, peak } = fakeDeps()
    await run(deps, ['a', 'b', 'c', 'd', 'e'])
    expect(peak()).toBeLessThanOrEqual(2)
  })

  it('reports the stage of every device in order', async () => {
    const { deps } = fakeDeps()
    const seen: string[] = []
    await run(deps, ['a', 'b'], () => false, (id, s) => seen.push(`${id}:${s}`))
    expect(seen.slice(0, 3)).toEqual(['a:waiting', 'b:waiting', 'a:building'])
    expect(seen.filter((s) => s.startsWith('b:'))).toEqual(['b:waiting', 'b:installing', 'b:variant', 'b:launching', 'b:capturing', 'b:done'])
  })

  it('does nothing without devices', async () => {
    const { deps, calls } = fakeDeps()
    expect(await run(deps, [])).toEqual({ shots: [], restoreFailed: [] })
    expect(calls).toEqual([])
  })
})
