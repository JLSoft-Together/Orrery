import { mkdirSync, mkdtempSync, realpathSync, rmSync, utimesSync, writeFileSync } from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { parseCleanCommand, type CleanItem, type DiskHistoryEntry } from '@ccm/shared'
import { DiskCleanService, type DiskCleanDeps } from '../src/diskClean/service'
import { androidUse, avdRunning, parseProcLines, readBuildFile, readWrapper, sdkSpots, versionCmp, type AndroidUse, type SdkDeps } from '../src/diskClean/sdk'
import { dockerBytes, dockerProbe, parseDockerDf, type DockerRun } from '../src/diskClean/docker'
import { commandBoxOptions } from '../src/diskClean/dialog'
import { macTempTree, resolveDeletable, type CanonicalIo } from '../src/diskMap/guard'
import { macTempAllow, sysRoots } from '../src/diskClean/system'

let tmp: string
beforeEach(() => {
  tmp = realpathSync(mkdtempSync(path.join(os.tmpdir(), 'ccm-sdk-')))
})
afterEach(() => {
  rmSync(tmp, { recursive: true, force: true })
})

const DAY = 86_400_000
function put(rel: string, text = 'x', ageDays = 0): string {
  const file = path.join(tmp, rel)
  mkdirSync(path.dirname(file), { recursive: true })
  writeFileSync(file, text)
  if (ageDays) {
    const t = new Date(Date.now() - ageDays * DAY)
    utimesSync(file, t, t)
  }
  return file
}
const dir = (rel: string): string => {
  const p = path.join(tmp, rel)
  mkdirSync(p, { recursive: true })
  return p
}

function sdk(over: Partial<SdkDeps> = {}): SdkDeps {
  return { sdkRoot: path.join(tmp, 'sdk'), avdHome: path.join(tmp, 'avd'), gradleHome: path.join(tmp, 'gradle'), xcodeBase: path.join(tmp, 'Xcode'), procs: async () => [], ...over }
}

function deps(over: Partial<DiskCleanDeps> = {}): DiskCleanDeps & { history: DiskHistoryEntry[] } {
  const history: DiskHistoryEntry[] = []
  return {
    platform: process.platform,
    home: path.join(tmp, 'home'),
    env: {},
    self: () => [],
    running: () => [],
    keep: () => [],
    emit: () => undefined,
    confirm: async () => true,
    trash: async (p) => rmSync(p, { recursive: true, force: true }),
    remove: async (p) => rmSync(p, { recursive: true, force: true }),
    record: async (e) => {
      history.push(e)
    },
    history,
    ...over,
  }
}

const byKind = (items: CleanItem[], kind: string): CleanItem[] => items.filter((i) => i.kind === kind)

function fakeSdk(): void {
  for (const api of ['android-30', 'android-33', 'android-34', 'android-35']) put(`sdk/platforms/${api}/android.jar`, 'jar')
  for (const v of ['30.0.3', '33.0.1', '34.0.0', '35.0.0']) put(`sdk/build-tools/${v}/aapt`, 'aapt')
  put('sdk/system-images/android-34/google_apis/x86_64/system.img', 'img')
  put('sdk/system-images/android-30/default/x86/system.img', 'img')
  put('avd/Pixel_8.avd/config.ini', 'image.sysdir.1=system-images\\android-34\\google_apis\\x86_64\\\n')
  put('avd/Pixel_8.ini', 'path=x')
  put('avd/Old.avd/config.ini', 'image.sysdir.1=system-images/android-34/google_apis/x86_64/\n')
  put('avd/Old.ini', 'path=y')
}

describe('android project use', () => {
  it('reads compileSdk, buildToolsVersion and the wrapper', () => {
    const use: AndroidUse = { compile: new Set(), tools: new Set(), gradle: new Set(), android: false, compileUnsure: false, toolsUnsure: false, gradleUnsure: false }
    readBuildFile('plugins { id("com.android.application") }\nandroid {\n  compileSdk = 33\n  buildToolsVersion = "33.0.1"\n}', use)
    expect([...use.compile]).toEqual([33])
    expect([...use.tools]).toEqual(['33.0.1'])
    expect(use.compileUnsure).toBe(false)
    readBuildFile('android {\n compileSdk = flutter.compileSdkVersion\n}', use)
    expect(use.compileUnsure).toBe(true)
    expect(use.toolsUnsure).toBe(true)
    expect(readWrapper('distributionUrl=https\\://services.gradle.org/distributions/gradle-8.7-bin.zip')).toBe('gradle-8.7-bin')
  })

  it('treats no Android project as unsure', async () => {
    const use = await androidUse([dir('web')])
    expect(use.compileUnsure && use.toolsUnsure && use.gradleUnsure).toBe(true)
  })

  it('orders versions', () => {
    expect(['35.0.0', '30.0.3', '34.0.0-rc1', '34.0.0'].sort(versionCmp)).toEqual(['30.0.3', '34.0.0-rc1', '34.0.0', '35.0.0'])
  })

  it('finds the SDK from env first', () => {
    expect(sdkSpots({ platform: 'win32', home: 'C:\\Users\\a', env: { ANDROID_HOME: 'D:\\sdk' } }).sdkRoot).toBe('D:\\sdk')
    expect(sdkSpots({ platform: 'win32', home: 'C:\\Users\\a', env: { LOCALAPPDATA: 'C:\\Users\\a\\AppData\\Local' } }).sdkRoot).toBe('C:\\Users\\a\\AppData\\Local\\Android\\Sdk')
    expect(sdkSpots({ platform: 'darwin', home: '/Users/a', env: {} }).avdHome).toBe('/Users/a/.android/avd')
  })
})

describe('sdk providers', () => {
  it('keeps the newest and the project version, ticks only when sure', async () => {
    fakeSdk()
    put('proj/app/build.gradle.kts', 'plugins { id("com.android.application") }\nandroid {\n  compileSdk = 33\n  buildToolsVersion = "33.0.1"\n}')
    put('proj/gradle/wrapper/gradle-wrapper.properties', 'distributionUrl=https\\://x/gradle-7.5-bin.zip')
    for (const v of ['gradle-7.5-bin', 'gradle-8.2-all', 'gradle-8.9-bin']) put(`gradle/wrapper/dists/${v}/hash/gradle.zip`, 'z')
    const svc = new DiskCleanService(deps({ sdk: sdk() }))
    const res = await svc.scan({ ids: ['sdk', 'avd', 'gradle_dists'], roots: [path.join(tmp, 'proj')], open: [] })
    const platforms = byKind(res.items, 'platform').map((i) => i.detail).sort()
    expect(platforms).toEqual(['android-30', 'android-34'])
    expect(byKind(res.items, 'platform').every((i) => !i.hold && i.restore?.startsWith('sdkmanager "platforms;'))).toBe(true)
    expect(byKind(res.items, 'buildTools').map((i) => i.detail).sort()).toEqual(['30.0.3', '34.0.0'])
    expect(byKind(res.items, 'sysImage').map((i) => i.detail)).toEqual(['android-30;default;x86'])
    expect(byKind(res.items, 'gradleDist').map((i) => i.detail)).toEqual(['gradle-8.2-all'])
    const avds = res.items.filter((i) => i.category === 'avd')
    expect(avds.map((i) => i.detail).sort()).toEqual(['Old', 'Old.ini', 'Pixel_8', 'Pixel_8.ini'])
    expect(avds.every((i) => i.hold && !i.locked)).toBe(true)
  })

  it('holds platforms when a project version is unknown', async () => {
    fakeSdk()
    put('flutter/android/app/build.gradle', 'android {\n compileSdk = flutter.compileSdkVersion\n}')
    const res = await new DiskCleanService(deps({ sdk: sdk() })).scan({ ids: ['sdk'], roots: [path.join(tmp, 'flutter')], open: [] })
    const p = byKind(res.items, 'platform')
    expect(p.length).toBe(3)
    expect(p.every((i) => i.hold)).toBe(true)
  })

  it('locks a running AVD by lock file or emulator command line, and rechecks before delete', async () => {
    fakeSdk()
    put('avd/Pixel_8.avd/hardware-qemu.ini.lock', '')
    const procs = vi.fn(async () => parseProcLines('qemu-system-x86_64.exe\temulator -netdelay none @Old'))
    const svc = new DiskCleanService(deps({ sdk: sdk({ procs }) }))
    const res = await svc.scan({ ids: ['avd'], roots: [], open: [] })
    expect(res.items.every((i) => i.locked === 'app')).toBe(true)
    expect(avdRunning('Old', await procs())).toBe(true)
    expect(avdRunning('Pixel', await procs())).toBe(false)
    rmSync(path.join(tmp, 'avd', 'Pixel_8.avd', 'hardware-qemu.ini.lock'))
    procs.mockResolvedValue([])
    const res2 = await svc.scan({ ids: ['avd'], roots: [], open: [] })
    expect(res2.items.some((i) => i.locked)).toBe(false)
    put('avd/Old.avd/multiinstance.lock', '')
    const del = await svc.remove({ paths: [path.join(tmp, 'avd', 'Old.avd')], permanent: false, open: [] })
    expect(del.failed[0]?.reason).toBe('locked:app')
    const ok = await svc.remove({ paths: [path.join(tmp, 'avd', 'Pixel_8.avd'), path.join(tmp, 'avd', 'Pixel_8.ini')], permanent: false, open: [] })
    expect(ok.done.length).toBe(2)
  })

  it('notes a missing SDK', async () => {
    const res = await new DiskCleanService(deps({ sdk: sdk() })).scan({ ids: ['sdk'], roots: [], open: [] })
    expect(res.notes?.sdk).toBe('noSdk')
  })
})

const DF = [
  '{"Active":"2","Reclaimable":"1.5GB (60%)","Size":"2.5GB","TotalCount":"5","Type":"Images"}',
  '{"Active":"1","Reclaimable":"20MB (50%)","Size":"40MB","TotalCount":"3","Type":"Containers"}',
  '{"Active":"0","Reclaimable":"300kB (100%)","Size":"300kB","TotalCount":"2","Type":"Local Volumes"}',
  '{"Active":"0","Reclaimable":"0B","Size":"1.2GB","TotalCount":"9","Type":"Build Cache"}',
].join('\n')

function fakeDocker(mode: 'ok' | 'noCli' | 'noDaemon' | 'timeout'): { run: DockerRun; calls: string[][] } {
  const calls: string[][] = []
  let pruned = false
  const run: DockerRun = async (args) => {
    calls.push(args)
    if (mode === 'noCli') return { ok: false, code: 'ENOENT', stdout: '' }
    if (mode === 'timeout') return { ok: false, code: 'ETIMEDOUT', stdout: '' }
    if (args[0] === 'info') return mode === 'ok' ? { ok: true, stdout: '27.0.1\n' } : { ok: false, code: 'EXIT', stdout: '' }
    if (args[0] === 'system') return { ok: true, stdout: pruned ? DF.replace('1.5GB (60%)', '0B (0%)') : DF }
    pruned = true
    return { ok: true, stdout: 'Total reclaimed space: 1.5GB' }
  }
  return { run, calls }
}

describe('docker provider', () => {
  it('parses sizes and df lines', () => {
    expect(dockerBytes('1.5GB (60%)')).toBe(1_500_000_000)
    expect(dockerBytes('300kB')).toBe(300_000)
    expect(dockerBytes('0B')).toBe(0)
    expect(parseDockerDf(DF).map((r) => r.kind)).toEqual(['images', 'containers', 'volumes', 'buildCache'])
  })

  it('hides the group with a reason when docker cannot answer', async () => {
    for (const [mode, note] of [
      ['noCli', 'noCli'],
      ['noDaemon', 'noDaemon'],
      ['timeout', 'timeout'],
    ] as const) {
      const { run } = fakeDocker(mode)
      expect(await dockerProbe({ run })).toBe(note)
      const res = await new DiskCleanService(deps({ docker: { run } })).scan({ ids: ['docker'], roots: [], open: [] })
      expect(res.items).toEqual([])
      expect(res.notes?.docker).toBe(note)
    }
  })

  it('lists one row per type, volumes held and warned, prunes only with the fixed official command after asking', async () => {
    const { run, calls } = fakeDocker('ok')
    const askCommand = vi.fn(async () => true)
    const d = deps({ docker: { run }, askCommand })
    const svc = new DiskCleanService(d)
    const res = await svc.scan({ ids: ['docker'], roots: [], open: [] })
    expect(res.items.map((i) => [i.kind, i.size, i.action])).toEqual([
      ['images', 1_500_000_000, 'command'],
      ['containers', 20_000_000, 'command'],
      ['volumes', 300_000, 'command'],
      ['buildCache', 0, 'command'],
    ])
    expect(res.items.find((i) => i.kind === 'volumes')).toMatchObject({ hold: true, warn: true })
    const del = await svc.remove({ paths: ['docker:images'], permanent: true, open: [] })
    expect(del.failed[0]?.reason).toBe('invalid')
    expect(await svc.runCommand('docker:nope')).toMatchObject({ error: 'invalid' })
    const r = await svc.runCommand('docker:images')
    expect(r).toEqual({ done: true, freed: 1_500_000_000 })
    expect(calls.find((c) => c[0] === 'image')).toEqual(['image', 'prune', '-a', '-f'])
    expect(askCommand.mock.calls[0]![0]).toMatchObject({ kind: 'images', command: 'docker image prune -a -f', warn: false })
    expect(d.history[0]).toMatchObject({ action: 'delete', paths: ['docker:images'] })
    askCommand.mockResolvedValueOnce(false)
    expect(await svc.runCommand('docker:volumes')).toMatchObject({ cancelled: true })
    expect(calls.some((c) => c[0] === 'volume')).toBe(false)
    expect(parseCleanCommand({ path: 'docker:volumes' })).toEqual({ path: 'docker:volumes' })
    expect(parseCleanCommand({ path: 'a\nb' })).toBeNull()
  })

  it('builds a dialog that warns for volumes', () => {
    const box = commandBoxOptions({ kind: 'volumes', bytes: 300_000, count: 2, command: 'docker volume prune -f', warn: true }, 'vi')
    expect(box.detail).toContain('mất hẳn')
    expect(box.cancelId).toBe(1)
  })
})

describe('macOS TMPDIR guard', () => {
  const T = '/private/var/folders/ab/xyz123/T'
  const io: CanonicalIo = {
    lstat: async () => ({ isSymbolicLink: () => false }),
    realpath: async (p) => p.replace(/^\/var\//, '/private/var/'),
  }
  const ctx = { platform: 'darwin' as const, home: '/Users/me', keep: [], roots: ['/var/folders/ab/xyz123/T/old.log'] }

  it('blocks /private/var/folders without the allow list', async () => {
    expect((await resolveDeletable('/var/folders/ab/xyz123/T/old.log', ctx, io)).reason).toBe('system')
  })

  it('allows children of the real user TMPDIR only', async () => {
    const allow = { ...ctx, allowTree: [T] }
    expect((await resolveDeletable('/var/folders/ab/xyz123/T/old.log', allow, io)).reason).toBeNull()
    expect((await resolveDeletable('/var/folders/ab/xyz123/T', { ...allow, roots: ['/var/folders/ab/xyz123/T'] }, io)).reason).toBe('system')
    expect((await resolveDeletable('/private/var/folders/ab/other/T/x', { ...allow, roots: ['/private/var/folders/ab/other/T/x'] }, io)).reason).toBe('system')
    expect((await resolveDeletable('/private/etc/hosts', { ...ctx, roots: ['/private/etc/hosts'], allowTree: ['/private'] }, io)).reason).toBe('system')
  })

  it('accepts only the per-user T / C shape', async () => {
    expect(macTempTree('/private/var/folders/ab/xyz123/T/')).toBe(T)
    expect(macTempTree('/private/var/folders/ab')).toBeNull()
    expect(macTempTree('/private/tmp')).toBeNull()
    const roots = sysRoots({ platform: 'darwin', home: '/Users/me', env: { TMPDIR: '/var/folders/ab/xyz123/T/' } })
    expect(await macTempAllow(roots, async (p) => p.replace(/^\/var\//, '/private/var/'))).toEqual([T, '/private/var/folders/ab/xyz123/C'])
    expect(await macTempAllow(sysRoots({ platform: 'win32', home: 'C:\\Users\\me', env: {} }))).toEqual([])
  })
})
