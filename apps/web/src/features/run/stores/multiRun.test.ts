import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import type { RunSession } from '@ccm/shared'
import type { MultiDeps } from '../lib/multiRun'

vi.mock('../lib/multiRunDeps', () => ({
  buildDeps: (_cfg: unknown, buildOn: (id: string) => Promise<{ ok: boolean; failure: { code: string } | null }>): MultiDeps => ({
    build: async (id) => {
      const r = await buildOn(id)
      return r.ok ? { ok: true, apk: 'a.apk', appId: 'app', activity: null } : { ok: false, error: r.failure?.code ?? 'build' }
    },
    install: async () => ({ ok: true }),
    open: async () => ({ ok: true }),
    apply: async () => ({ ok: true }),
    restore: async () => ({ ok: true }),
    capture: async () => ({ ok: true, png: 'p', width: 1, height: 1 }),
    wait: async () => undefined,
  }),
}))

import { useRunStore } from './run'
import { SESSION_MAX_MS, useMultiRunStore } from './multiRun'

const session = (state: RunSession['state']): RunSession => ({ id: 's1', configId: 'c1', configName: 'app', type: 'android', cwd: 'D:/p', deviceId: 'd1', deviceName: 'd1', mode: null, state, startedAt: 1, endedAt: null }) as RunSession

function setup() {
  setActivePinia(createPinia())
  const run = useRunStore()
  const multi = useMultiRunStore()
  run.configsFor = (() => [{ id: 'c1', type: 'android', cwd: 'D:/p', module: 'app', variant: 'debug' }]) as never
  run.startConfig = vi.fn(async () => {
    run.sessions.s1 = session('building')
    return { ok: true, sessionId: 's1' }
  }) as never
  run.stop = vi.fn(async () => undefined) as never
  return { run, multi }
}

beforeEach(() => vi.useFakeTimers())

describe('multiRun settle', () => {
  it('a session that disappears mid build ends the device with session-missing', async () => {
    const { run, multi } = setup()
    const p = multi.startMany('D:/p', 'c1', ['d1'], 'light')
    await vi.advanceTimersByTimeAsync(10)
    delete run.sessions.s1
    await vi.advanceTimersByTimeAsync(10)
    const out = await p
    expect(out?.shots[0]?.error).toBe('session-missing')
  })

  it('a session stuck before running hits the max wait instead of hanging', async () => {
    const { multi } = setup()
    const p = multi.startMany('D:/p', 'c1', ['d1'], 'light')
    await vi.advanceTimersByTimeAsync(SESSION_MAX_MS + 100)
    const out = await p
    expect(out?.shots[0]?.error).toBe('timeout')
  })
})
