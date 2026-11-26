import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { AndroidRunConfig } from '@ccm/shared'

const api = vi.hoisted(() => ({
  built: vi.fn(),
  install: vi.fn(),
  launch: vi.fn(),
  manifest: vi.fn(),
  shot: vi.fn(),
  variant: vi.fn(),
}))

vi.mock('../../devices/lib/devices', () => ({
  devBuiltApk: (...a: unknown[]) => api.built(...a),
  devInstall: (...a: unknown[]) => api.install(...a),
  devLaunch: (...a: unknown[]) => api.launch(...a),
  devManifest: (...a: unknown[]) => api.manifest(...a),
  devScreencap: (...a: unknown[]) => api.shot(...a),
  devVariant: (...a: unknown[]) => api.variant(...a),
}))

import { buildDeps, issueText, sweepVariants } from './multiRunDeps'

const cfg = { id: 'c', type: 'android', name: 'app', cwd: '/p', module: ':app', variant: 'debug' } as unknown as AndroidRunConfig
const wait = async (): Promise<void> => {}

beforeEach(() => {
  for (const f of Object.values(api)) f.mockReset()
  api.built.mockResolvedValue({ ok: true, apk: '/p/app.apk', appId: 'dev.x' })
  api.manifest.mockResolvedValue({ ok: true, activities: [{ activity: 'dev.x.Other', launcher: false }, { activity: 'dev.x.Main', launcher: true }] })
})

describe('multiRunDeps', () => {
  it('a good first run returns the apk, the app id and the launcher activity', async () => {
    const deps = buildDeps(cfg, async () => ({ ok: true, failure: null }), wait)
    expect(await deps.build('a')).toEqual({ ok: true, apk: '/p/app.apk', appId: 'dev.x', activity: 'dev.x.Main' })
    expect(api.built).toHaveBeenCalledWith('/p', ':app', 'debug')
  })

  it('a build failure is not given the stale apk, an install-only failure is', async () => {
    const deps = buildDeps(cfg, async () => ({ ok: false, failure: { code: 'build' } }), wait)
    expect(await deps.build('a')).toEqual({ ok: false, error: 'build' })
    expect(api.built).not.toHaveBeenCalled()
    const inst = buildDeps(cfg, async () => ({ ok: false, failure: { code: 'install', detail: 'signature' } }), wait)
    expect(await inst.build('a')).toMatchObject({ ok: false, error: 'install-signature', built: { apk: '/p/app.apk' } })
    const none = buildDeps(cfg, async () => ({ ok: false, failure: { code: 'install', detail: 'apk-missing' } }), wait)
    expect(await none.build('a')).toEqual({ ok: false, error: 'install-apk-missing' })
  })

  it('opening needs a launcher activity', async () => {
    const deps = buildDeps(cfg, async () => ({ ok: true, failure: null }), wait)
    expect(await deps.open('b', { apk: 'x', appId: 'dev.x', activity: null })).toEqual({ ok: false, error: 'no-launcher' })
    api.launch.mockResolvedValue({ ok: true })
    expect(await deps.open('b', { apk: 'x', appId: 'dev.x', activity: 'dev.x.Main' })).toEqual({ ok: true })
    expect(api.launch).toHaveBeenCalledWith('b', { kind: 'activity', pkg: 'dev.x', activity: 'dev.x.Main' })
  })

  it('maps install, variant and capture results', async () => {
    const deps = buildDeps(cfg, async () => ({ ok: true, failure: null }), wait)
    api.install.mockResolvedValue({ ok: false, error: 'storage' })
    expect(await deps.install('b', '/p/app.apk')).toEqual({ ok: false, error: 'storage' })
    api.variant.mockResolvedValue({ ok: true })
    await deps.apply('b', { id: 'dark', night: true, fontScale: null })
    expect(api.variant).toHaveBeenLastCalledWith({ op: 'apply', id: 'b', night: true, fontScale: null })
    await deps.restore('b')
    expect(api.variant).toHaveBeenLastCalledWith({ op: 'restore', id: 'b' })
    api.shot.mockResolvedValue({ ok: true, png: 'p', width: 1, height: 2 })
    expect(await deps.capture('b')).toEqual({ ok: true, png: 'p', width: 1, height: 2 })
    api.shot.mockResolvedValue({ ok: false, error: 'secure' })
    expect(await deps.capture('b')).toEqual({ ok: false, error: 'secure' })
  })

  it('sweeps pending restores and never throws', async () => {
    api.variant.mockRejectedValue(new Error('x'))
    await expect(sweepVariants()).resolves.toBeUndefined()
    expect(api.variant).toHaveBeenCalledWith({ op: 'sweep' })
    expect(issueText(null)).toBe('build')
  })
})
