import { beforeEach, describe, expect, it } from 'vitest'
import type { AdbDevice, AvdInfo, RemoteDevice } from '@ccm/shared'
import { choose, closeNames, countFilters, emptyChoices, forgetChoice, loadChoices, matchesFilter, mergeDevices, pairKey, parseChoices, saveChoices, sameModel, MERGE_STORAGE_KEY, type MergeInput } from './mergeDevices'
import { compareVersions, isOlder, rowAction, type ActionContext } from './deviceActions'

const remote = (over: Partial<RemoteDevice> = {}): RemoteDevice => ({ deviceId: 'r1', name: 'Galaxy A33', platform: 'android', pairedAt: 1, lastSeen: 2, state: 'connected', model: 'SM-A336E', ...over })
const adb = (over: Partial<AdbDevice> = {}): AdbDevice => ({ id: 'R58M', state: 'ready', transport: 'usb', model: 'SM_A336E', product: null, device: null, transportId: null, alias: null, name: 'SM_A336E', online: true, lastSeen: null, ...over })
const avd = (over: Partial<AvdInfo> = {}): AvdInfo => ({ name: 'Medium_Phone_API_35', displayName: 'Medium Phone API 35', api: 35, abi: 'arm64-v8a', device: 'pixel_7', tag: null, serial: null, ...over })

const base = (over: Partial<MergeInput> = {}): MergeInput => ({ remote: [], adb: [], avds: [], hashes: {}, choices: emptyChoices(), ...over })

describe('mergeDevices', () => {
  it('merges a remote device and an adb device with the same serial hash into one row', () => {
    const out = mergeDevices(base({ remote: [remote({ adbSerialHash: 'abcdef0123456789' })], adb: [adb()], hashes: { R58M: 'abcdef0123456789' } }))
    expect(out.rows).toHaveLength(1)
    expect(out.rows[0]).toMatchObject({ kind: 'phone', link: 'hash', suggestion: null })
    expect(out.rows[0]?.remote?.deviceId).toBe('r1')
    expect(out.rows[0]?.adb?.id).toBe('R58M')
    expect(out.suggestions).toEqual([])
  })

  it('matches the hash ignoring case', () => {
    const out = mergeDevices(base({ remote: [remote({ adbSerialHash: 'ABCDEF0123456789' })], adb: [adb()], hashes: { R58M: 'abcdef0123456789' } }))
    expect(out.rows).toHaveLength(1)
  })

  it('suggests a merge for the same model with a close name and keeps two rows', () => {
    const out = mergeDevices(base({ remote: [remote()], adb: [adb({ name: 'A33 work phone' })], hashes: { R58M: 'ffff' } }))
    expect(out.rows).toHaveLength(2)
    expect(out.suggestions).toEqual([{ key: pairKey('r1', 'R58M'), remoteId: 'r1', adbId: 'R58M', remoteName: 'Galaxy A33', adbName: 'A33 work phone' }])
    expect(out.rows.find((r) => r.remote)?.suggestion?.adbId).toBe('R58M')
  })

  it('offers one adb phone to only one of two remotes with the same model', () => {
    const out = mergeDevices(base({ remote: [remote(), remote({ deviceId: 'r2', name: 'Galaxy A33 spare' })], adb: [adb({ name: 'A33 work phone' })] }))
    expect(out.suggestions).toHaveLength(1)
    expect(out.rows.filter((r) => r.suggestion)).toHaveLength(1)
  })

  it('never merges by name alone: different model, same name', () => {
    const out = mergeDevices(base({ remote: [remote({ name: 'Pixel 8', model: 'Pixel 8' })], adb: [adb({ name: 'Pixel 8', model: 'Redmi Note 12' })] }))
    expect(out.rows).toHaveLength(2)
    expect(out.suggestions).toEqual([])
  })

  it('never merges by model alone: same model, unrelated name', () => {
    const out = mergeDevices(base({ remote: [remote({ name: 'Kitchen tablet' })], adb: [adb({ name: 'Desk unit' })] }))
    expect(out.rows).toHaveLength(2)
    expect(out.suggestions).toEqual([])
  })

  it('remembers Merge: the pair becomes one row with link chosen', () => {
    const out = mergeDevices(base({ remote: [remote()], adb: [adb({ name: 'A33 work phone' })], choices: choose(emptyChoices(), pairKey('r1', 'R58M'), true) }))
    expect(out.rows).toHaveLength(1)
    expect(out.rows[0]?.link).toBe('chosen')
    expect(out.suggestions).toEqual([])
  })

  it('remembers Keep apart: no suggestion again', () => {
    const out = mergeDevices(base({ remote: [remote()], adb: [adb({ name: 'A33 work phone' })], choices: choose(emptyChoices(), pairKey('r1', 'R58M'), false) }))
    expect(out.rows).toHaveLength(2)
    expect(out.suggestions).toEqual([])
  })

  it('a choice replaces the opposite one and can be forgotten', () => {
    const k = pairKey('r1', 'R58M')
    const both = choose(choose(emptyChoices(), k, true), k, false)
    expect(both).toEqual({ merged: [], split: [k] })
    expect(forgetChoice(both, k)).toEqual(emptyChoices())
  })

  it('persists choices in localStorage under ccm.deviceMerge and survives bad data', () => {
    localStorage.removeItem(MERGE_STORAGE_KEY)
    expect(loadChoices()).toEqual(emptyChoices())
    saveChoices({ merged: ['a|b'], split: ['c|d'] })
    expect(localStorage.getItem('ccm.deviceMerge')).toContain('a|b')
    expect(loadChoices()).toEqual({ merged: ['a|b'], split: ['c|d'] })
    expect(parseChoices('{nope')).toEqual(emptyChoices())
    expect(parseChoices('{"merged":[1,"x"],"split":"y"}')).toEqual({ merged: ['x'], split: [] })
  })

  it('one adb device is claimed by one remote device only', () => {
    const out = mergeDevices(base({ remote: [remote({ deviceId: 'r1', adbSerialHash: 'aa' }), remote({ deviceId: 'r2', adbSerialHash: 'aa' })], adb: [adb()], hashes: { R58M: 'aa' } }))
    expect(out.rows.filter((r) => r.adb)).toHaveLength(1)
    expect(out.rows).toHaveLength(2)
  })

  it('drops revoked remote devices and keeps revoking ones', () => {
    const out = mergeDevices(base({ remote: [remote({ deviceId: 'x', state: 'revoked' }), remote({ deviceId: 'y', revoking: true })] }))
    expect(out.rows.map((r) => r.remote?.deviceId)).toEqual(['y'])
  })

  it('lists an offline AVD, and one row per running emulator with its AVD name', () => {
    const off = mergeDevices(base({ avds: [avd()] }))
    expect(off.rows).toHaveLength(1)
    expect(off.rows[0]).toMatchObject({ kind: 'emulator', adb: null, name: 'Medium Phone API 35' })
    const on = mergeDevices(base({ avds: [avd({ serial: 'emulator-5554' })], adb: [adb({ id: 'emulator-5554', transport: 'emulator', name: 'sdk_gphone' })] }))
    expect(on.rows).toHaveLength(1)
    expect(on.rows[0]).toMatchObject({ kind: 'emulator', key: 'v:Medium_Phone_API_35' })
    expect(on.rows[0]?.adb?.id).toBe('emulator-5554')
  })

  it('does not suggest merging a remote device into an emulator', () => {
    const out = mergeDevices(base({ remote: [remote({ model: 'sdk_gphone' , name: 'sdk gphone' })], adb: [adb({ id: 'emulator-5554', transport: 'emulator', model: 'sdk_gphone', name: 'sdk gphone' })] }))
    expect(out.suggestions).toEqual([])
  })

  it('a desktop without the new fields still lists remote devices', () => {
    const old = remote()
    delete old.pairedVia
    delete old.adbSerialHash
    delete old.revoking
    delete old.appVersion
    const out = mergeDevices(base({ remote: [old], adb: [adb({ model: 'Other' , name: 'Other' })] }))
    expect(out.rows).toHaveLength(2)
  })

  it('puts online rows first then sorts by name', () => {
    const out = mergeDevices(base({ remote: [remote({ deviceId: 'z', name: 'Zed', state: 'connected' }), remote({ deviceId: 'a', name: 'Alpha', state: 'idle' }), remote({ deviceId: 'm', name: 'Mid', state: 'connected' })] }))
    expect(out.rows.map((r) => r.name)).toEqual(['Mid', 'Zed', 'Alpha'])
  })

  it('filters and counts by role', () => {
    const out = mergeDevices(base({ remote: [remote({ adbSerialHash: 'aa' })], adb: [adb(), adb({ id: 'WIFI', transport: 'wifi', model: 'X', name: 'X' })], hashes: { R58M: 'aa' }, avds: [avd()] }))
    expect(countFilters(out.rows)).toEqual({ all: 3, remote: 1, adb: 2, emulator: 1 })
    expect(out.rows.filter((r) => matchesFilter(r, 'emulator'))).toHaveLength(1)
  })
})

describe('similarity helpers', () => {
  it('compares models loosely but needs four characters', () => {
    expect(sameModel('Samsung SM-A336E', 'SM_A336E')).toBe(true)
    expect(sameModel('A33', 'A33')).toBe(false)
    expect(sameModel(undefined, 'SM_A336E')).toBe(false)
  })

  it('treats names as close when they share a token or one contains the other', () => {
    expect(closeNames('Galaxy A33', 'A33 work phone')).toBe(true)
    expect(closeNames('SM-A336E', 'sm_a336e')).toBe(true)
    expect(closeNames('Kitchen tablet', 'Desk unit')).toBe(false)
  })
})

describe('versions and actions', () => {
  it('compares dotted versions numerically', () => {
    expect(compareVersions('4.0.8', '4.0.10')).toBe(-1)
    expect(compareVersions('4.1', '4.0.10')).toBe(1)
    expect(compareVersions('4.0', '4.0.0')).toBe(0)
    expect(compareVersions('abc', '1.0')).toBeNull()
    expect(isOlder('4.0.8+12', '4.0.10')).toBe(true)
  })

  const ctx = (over: Partial<ActionContext> = {}): ActionContext => ({ desktopVersion: '4.0.10', pkg: {}, canInstall: true, ...over })
  const row = (r: MergeInput) => mergeDevices(r).rows[0]!

  it('offers Update when the installed version is older than the desktop', () => {
    const r = row(base({ adb: [adb()] }))
    expect(rowAction(r, ctx({ pkg: { R58M: { ok: true, installed: true, versionName: '4.0.8', versionCode: 8 } } }))).toBe('update')
    expect(rowAction(r, ctx({ pkg: { R58M: { ok: true, installed: true, versionName: '4.0.10', versionCode: 10 } } }))).toBeNull()
  })

  it('offers no update when the desktop version is unknown', () => {
    const r = row(base({ adb: [adb()] }))
    expect(rowAction(r, ctx({ desktopVersion: '', pkg: { R58M: { ok: true, installed: true, versionName: '4.0.8', versionCode: 8 } } }))).toBeNull()
  })

  it('offers Install when Remote is missing and waits while the version is unknown', () => {
    const r = row(base({ adb: [adb()] }))
    expect(rowAction(r, ctx({ pkg: { R58M: { ok: true, installed: false } } }))).toBe('install')
    expect(rowAction(r, ctx())).toBeNull()
    expect(rowAction(r, ctx({ pkg: { R58M: { ok: false, error: 'denied' } } }))).toBeNull()
  })

  it('blocks Install on an x86_64 emulator and allows it on arm64', () => {
    const emu = (abi: string) => row(base({ avds: [avd({ abi, serial: 'emulator-5554' })], adb: [adb({ id: 'emulator-5554', transport: 'emulator' })] }))
    const pkg = { 'emulator-5554': { ok: true as const, installed: false as const } }
    expect(rowAction(emu('x86_64'), ctx({ pkg }))).toBe('blocked')
    expect(rowAction(emu('arm64-v8a'), ctx({ pkg }))).toBe('install')
  })

  it('offers Start for a stopped emulator and nothing while a phone is revoking', () => {
    expect(rowAction(row(base({ avds: [avd()] })), ctx())).toBe('boot')
    expect(rowAction(row(base({ remote: [remote({ revoking: true })] })), ctx())).toBeNull()
  })

  it('offers nothing for an offline or unauthorized adb device', () => {
    const pkg = { R58M: { ok: true as const, installed: false as const } }
    expect(rowAction(row(base({ adb: [adb({ state: 'unauthorized' })] })), ctx({ pkg }))).toBeNull()
    expect(rowAction(row(base({ adb: [adb({ online: false })] })), ctx({ pkg }))).toBeNull()
    expect(rowAction(row(base({ adb: [adb()] })), ctx({ pkg, canInstall: false }))).toBeNull()
  })
})

beforeEach(() => localStorage.removeItem(MERGE_STORAGE_KEY))
