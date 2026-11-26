import { beforeEach, describe, expect, it } from 'vitest'
import type { RunSession } from '@ccm/shared'
import { apkOf, autoApk, autoApkOn, setAutoApk, shouldAutoOffer } from './runApk'

const session = (over: Record<string, unknown> = {}): RunSession =>
  ({ id: 's1', configId: 'c', configName: 'c', type: 'android', cwd: 'C:/work/app', deviceId: null, deviceName: null, mode: null, state: 'installing', startedAt: 1, endedAt: null, appId: null, vmServiceUri: null, devtoolsUri: null, failure: null, progress: null, ...over }) as RunSession

const withApk = (over: Record<string, unknown> = {}): RunSession => session({ apk: { path: 'x', size: 10, applicationId: 'com.ex', variant: 'debug' }, ...over })

beforeEach(() => {
  localStorage.clear()
  autoApk.value = {}
})

describe('apkOf', () => {
  it('reads the patched apk and is null without it', () => {
    expect(apkOf(withApk())).toEqual({ size: 10, applicationId: 'com.ex', variant: 'debug' })
    expect(apkOf(session())).toBeNull()
    expect(apkOf(null)).toBeNull()
    expect(apkOf(session({ apk: 'x' }))).toBeNull()
  })
})

describe('shouldAutoOffer', () => {
  const roots = ['C:/work/app']

  it('fires on the first event that carries the apk inside a switched-on root', () => {
    const offered = new Set<string>()
    expect(shouldAutoOffer(session({ state: 'building' }), withApk(), roots, offered)).toBe(true)
    expect(shouldAutoOffer(undefined, withApk(), roots, offered)).toBe(true)
  })

  it('does not fire again, after a reload, outside the project, or without the switch', () => {
    expect(shouldAutoOffer(withApk(), withApk({ state: 'running' }), roots, new Set())).toBe(false)
    expect(shouldAutoOffer(undefined, withApk(), roots, new Set(['s1']))).toBe(false)
    expect(shouldAutoOffer(undefined, withApk({ cwd: 'D:/other' }), roots, new Set())).toBe(false)
    expect(shouldAutoOffer(undefined, withApk(), [], new Set())).toBe(false)
    expect(shouldAutoOffer(undefined, session(), roots, new Set())).toBe(false)
  })
})

describe('auto switch', () => {
  it('is per project and remembered', () => {
    expect(autoApkOn('C:/work/app')).toBe(false)
    setAutoApk('C:/work/app', true)
    expect(autoApkOn('C:/work/app')).toBe(true)
    expect(autoApkOn('C:/work/other')).toBe(false)
    expect(JSON.parse(localStorage.getItem('ccm.run.apkAuto') ?? '{}')).toEqual({ [Object.keys(autoApk.value)[0]!]: 'C:/work/app' })
    setAutoApk('C:/work/app', false)
    expect(autoApkOn('C:/work/app')).toBe(false)
  })
})
