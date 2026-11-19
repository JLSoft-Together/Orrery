import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'
import type { InstalledApp } from '@ccm/shared'
import { appsFromKeys, dedupeApps, decodeReg, hiddenEntry, parseRegExport, readWindowsApps, REG_SOURCES } from '../src/diskApps/registry'
import { parseUninstall } from '../src/diskApps/uninstall'
import { blockedMac, lastUsedOf, parsePlistStrings, plistInfo } from '../src/diskApps/mac'
import { appKey, findLeftovers, publisherKey, type DirEntry, type LeftoverIo } from '../src/diskApps/leftovers'
import { DiskAppsService, type DiskAppsDeps } from '../src/diskApps/service'

const HKLM = String.raw`HKEY_LOCAL_MACHINE\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall`
const WOW = String.raw`HKEY_LOCAL_MACHINE\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall`

const hex2 = (s: string): string =>
  [...Buffer.from(`${s}\0`, 'utf16le')].map((b) => b.toString(16).padStart(2, '0')).join(',')

const FIXTURE = [
  'Windows Registry Editor Version 5.00',
  '',
  `[${HKLM}]`,
  '',
  String.raw`[${HKLM}\7-Zip]`,
  '"DisplayName"="7-Zip 23.01 (x64)"',
  '"DisplayVersion"="23.01"',
  '"Publisher"="Igor Pavlov"',
  String.raw`"InstallLocation"="C:\\Program Files\\7-Zip\\"`,
  String.raw`"UninstallString"="\"C:\\Program Files\\7-Zip\\Uninstall.exe\""`,
  '"EstimatedSize"=dword:00001652',
  '"InstallDate"="20250405"',
  '',
  String.raw`[${HKLM}\{11111111-2222-3333-4444-555555555555}]`,
  '"DisplayName"="Cốc Cốc"',
  '"Publisher"="Công ty TNHH Cốc Cốc"',
  `"UninstallString"=hex(2):${hex2('MsiExec.exe /I{11111111-2222-3333-4444-555555555555}').slice(0, 59)},\\`,
  `  ${hex2('MsiExec.exe /I{11111111-2222-3333-4444-555555555555}').slice(60)}`,
  '',
  String.raw`[${HKLM}\HiddenRuntime]`,
  '"DisplayName"="Hidden runtime"',
  '"SystemComponent"=dword:00000001',
  '',
  String.raw`[${HKLM}\KB5000001]`,
  '"DisplayName"="Update for Thing (KB5000001)"',
  '"ParentKeyName"="Thing"',
  '',
  String.raw`[${HKLM}\NoName]`,
  '"Publisher"="Nobody"',
  '',
  String.raw`[${HKLM}\7-Zip\Nested]`,
  '"DisplayName"="Nested should not count"',
  '',
].join('\r\n')

describe('registry export parser', () => {
  it('reads strings, dwords, expand strings split over lines and unicode names', () => {
    const keys = parseRegExport(FIXTURE)
    const zip = keys.find((k) => k.key.endsWith(String.raw`\7-Zip`))!
    expect(zip.values.get('installlocation')).toBe(String.raw`C:\Program Files\7-Zip\\`.slice(0, -1))
    expect(zip.values.get('uninstallstring')).toBe(String.raw`"C:\Program Files\7-Zip\Uninstall.exe"`)
    expect(zip.values.get('estimatedsize')).toBe(0x1652)
    const coc = keys.find((k) => k.key.includes('{1111'))!
    expect(coc.values.get('displayname')).toBe('Cốc Cốc')
    expect(coc.values.get('uninstallstring')).toBe('MsiExec.exe /I{11111111-2222-3333-4444-555555555555}')
  })

  it('decodes UTF-16 LE files with BOM', () => {
    const buf = Buffer.concat([Buffer.from([0xff, 0xfe]), Buffer.from('Windows Registry Editor Version 5.00\r\n"A"="Liên Minh"', 'utf16le')])
    expect(decodeReg(buf)).toContain('Liên Minh')
  })

  it('hides system components, updates, nameless and nested keys', () => {
    const apps = appsFromKeys(parseRegExport(FIXTURE), REG_SOURCES[0]!, { ProgramFiles: 'C:\\Program Files' })
    expect(apps.map((a) => a.name).sort()).toEqual(['7-Zip 23.01 (x64)', 'Cốc Cốc'])
    const zip = apps.find((a) => a.name.startsWith('7-Zip'))!
    expect(zip.location).toBe(String.raw`C:\Program Files\7-Zip`)
    expect(zip.size).toBe(0x1652 * 1024)
    expect(zip.sizeFrom).toBe('estimate')
    expect(zip.arch).toBe('64')
    expect(zip.installedAt).toBe(new Date(2025, 3, 5).getTime())
    expect(hiddenEntry(new Map([['displayname', 'Security Update for X'], ['releasetype', 'Security Update']]))).toBe(true)
  })

  it('keeps one row when the same app sits in the 64 and 32 bit views', async () => {
    const wow = FIXTURE.replaceAll(HKLM, WOW)
    const apps = await readWindowsApps(async () => [
      { src: REG_SOURCES[0]!, text: FIXTURE },
      { src: REG_SOURCES[1]!, text: wow },
    ], {})
    expect(apps!.filter((a) => a.name.startsWith('7-Zip'))).toHaveLength(1)
    expect(apps!.find((a) => a.name.startsWith('7-Zip'))!.arch).toBe('64')
    const a = { name: 'X', version: '1', publisher: null, uninstall: null, scope: 'machine', arch: '64' } as InstalledApp
    const b = { ...a, uninstall: 'x.exe', arch: '32' } as InstalledApp
    expect(dedupeApps([a, b])[0]!.arch).toBe('32')
  })
})

describe('UninstallString parser', () => {
  const files = new Set([
    String.raw`C:\Program Files\My App\uninst.exe`,
    String.raw`C:\Windows\System32\msiexec.exe`,
    String.raw`C:\Windows\System32\cmd.exe`,
    String.raw`C:\Windows\System32\rundll32.exe`,
    String.raw`C:\Users\me\AppData\Local\Programs\Thorium\Uninstall Thorium.exe`,
  ].map((f) => f.toLowerCase()))
  const deps = { exists: (f: string) => files.has(f.toLowerCase()), env: { SystemRoot: String.raw`C:\Windows`, LOCALAPPDATA: String.raw`C:\Users\me\AppData\Local` } }

  it('splits a quoted exe with spaces from its arguments', () => {
    expect(parseUninstall(String.raw`"C:\Program Files\My App\uninst.exe" /S --mode="all users"`, deps)).toEqual({
      file: String.raw`C:\Program Files\My App\uninst.exe`,
      args: '/S --mode="all users"',
      display: String.raw`"C:\Program Files\My App\uninst.exe" /S --mode="all users"`,
    })
  })

  it('finds an unquoted exe with spaces the way CreateProcess does', () => {
    const r = parseUninstall(String.raw`C:\Program Files\My App\uninst.exe /S`, deps)
    expect(r).toMatchObject({ file: String.raw`C:\Program Files\My App\uninst.exe`, args: '/S' })
  })

  it('expands env vars in the exe path', () => {
    const r = parseUninstall(String.raw`"%LOCALAPPDATA%\Programs\Thorium\Uninstall Thorium.exe" /currentuser`, deps)
    expect(r).toMatchObject({ file: String.raw`C:\Users\me\AppData\Local\Programs\Thorium\Uninstall Thorium.exe`, args: '/currentuser' })
  })

  it('turns MsiExec /I or /X with a GUID into System32 msiexec /X', () => {
    expect(parseUninstall('MsiExec.exe /I{11111111-2222-3333-4444-555555555555}', deps)).toMatchObject({ file: String.raw`C:\Windows\System32\msiexec.exe`, args: '/X{11111111-2222-3333-4444-555555555555}' })
    expect(parseUninstall('msiexec /x {aaaaaaaa-2222-3333-4444-555555555555}', deps)).toMatchObject({ args: '/X{AAAAAAAA-2222-3333-4444-555555555555}' })
    expect(parseUninstall('MsiExec.exe /i C:\\evil.msi', deps)).toEqual({ error: 'msi' })
  })

  it('refuses shell metacharacters, shells, missing exes and odd quotes', () => {
    expect(parseUninstall(String.raw`"C:\Program Files\My App\uninst.exe" & del C:\x`, deps)).toEqual({ error: 'unsafe' })
    expect(parseUninstall(String.raw`"C:\Program Files\My App\uninst.exe" | calc`, deps)).toEqual({ error: 'unsafe' })
    expect(parseUninstall(String.raw`cmd.exe /c del C:\x`, deps)).toEqual({ error: 'shell' })
    expect(parseUninstall(String.raw`"C:\Windows\System32\cmd.exe" /c rd /s`, deps)).toEqual({ error: 'shell' })
    files.add(String.raw`c:\program files\my app\uninst.bat`)
    expect(parseUninstall(String.raw`"C:\Program Files\My App\uninst.bat" /S`, deps)).toEqual({ error: 'shell' })
    expect(parseUninstall(String.raw`"C:\Gone\uninst.exe" /S`, deps)).toEqual({ error: 'missing' })
    expect(parseUninstall(String.raw`winget uninstall --id x`, deps)).toEqual({ error: 'missing' })
    expect(parseUninstall(String.raw`"C:\Program Files\My App\uninst.exe" /S "x`, deps)).toEqual({ error: 'unsafe' })
    expect(parseUninstall(String.raw`"C:\Program Files\My App\uninst.exe"/S`, deps)).toEqual({ error: 'unsafe' })
    expect(parseUninstall(String.raw`..\uninst.exe`, deps)).toEqual({ error: 'missing' })
    expect(parseUninstall('', deps)).toEqual({ error: 'empty' })
    expect(parseUninstall('a.exe\r\nb.exe', deps)).toEqual({ error: 'unsafe' })
  })

  it('allows rundll32 style uninstallers from System32', () => {
    const r = parseUninstall(String.raw`RunDll32 C:\PROGRA~1\x\setup.dll,Uninstall`, deps)
    expect('file' in r && r.file.toLowerCase()).toBe(String.raw`c:\windows\system32\rundll32.exe`)
  })
})

describe('macOS bundles', () => {
  const PLIST = `<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleDocumentTypes</key>
  <array><dict><key>CFBundleIdentifier</key><string>wrong.nested</string></dict></array>
  <key>CFBundleIdentifier</key>
  <string>com.example.Tool</string>
  <key>LSUIElement</key>
  <true/>
  <key>CFBundleName</key>
  <string>Tool &amp; Co</string>
  <key>CFBundleShortVersionString</key>
  <string>2.4.1</string>
</dict>
</plist>`

  it('reads top level keys only', () => {
    expect(plistInfo(parsePlistStrings(PLIST))).toEqual({ bundleId: 'com.example.Tool', name: 'Tool & Co', version: '2.4.1' })
  })

  it('blocks system apps, Apple apps and Orrery itself', () => {
    expect(blockedMac('/System/Applications/Notes.app', 'com.apple.Notes', null)).toBe('system')
    expect(blockedMac('/Applications/Safari.app', 'com.apple.Safari', null)).toBe('system')
    expect(blockedMac('/Applications/Orrery.app', 'dev.ccm.monitor', '/Applications/Orrery.app')).toBe('self')
    expect(blockedMac('/Applications/Tool.app', 'com.example.Tool', '/Applications/Orrery.app')).toBeNull()
  })

  it('reads mdls last used dates', () => {
    expect(lastUsedOf('2026-11-01 08:30:00 +0000\0(null)', 3)).toEqual([Date.parse('2026-11-01T08:30:00+00:00'), null, null])
  })
})

describe('leftovers', () => {
  const tree: Record<string, DirEntry[]> = {
    'R:/Roaming': [
      { name: 'Thorium', dir: true, link: false },
      { name: 'Microsoft', dir: true, link: false },
      { name: 'Google', dir: true, link: false },
      { name: 'EDRLab', dir: true, link: false },
      { name: 'thorium-updater', dir: true, link: false },
      { name: 'Code', dir: true, link: false },
      { name: 'LinkedThorium', dir: true, link: true },
    ],
    'R:/Roaming/EDRLab': [{ name: 'Thorium Reader', dir: true, link: false }],
    'R:/Roaming/Microsoft': [{ name: 'Thorium', dir: true, link: false }],
    'R:/Local': [
      { name: 'Google', dir: true, link: false },
      { name: 'Thorium.log', dir: false, link: false },
    ],
    'R:/Local/Google': [{ name: 'Chrome', dir: true, link: false }],
  }
  const io: LeftoverIo = { readdir: async (d) => tree[d.replace(/\\/g, '/')] ?? [], exists: async () => false }
  const roots = [
    { base: 'appdata' as const, dir: 'R:/Roaming' },
    { base: 'localappdata' as const, dir: 'R:/Local' },
  ]

  it('normalizes app and publisher names', () => {
    expect(appKey('7-Zip 23.01 (x64)')).toBe('7zip')
    expect(appKey('UltraViewer phiên bản 6.6.48')).toBe('ultraviewer')
    expect(appKey('Thorium 3.4.0')).toBe('thorium')
    expect(publisherKey('Công ty TNHH Cốc Cốc')).toBe('cốccốc')
    expect(publisherKey('Google LLC')).toBe('google')
  })

  it('matches by name, publisher folder and loose prefix, never shared vendor parents or links', async () => {
    const hits = await findLeftovers({ name: 'Thorium 3.4.0', publisher: 'EDRLab', bundleId: null, location: null, installed: false, others: [] }, roots, 'win32', io)
    const by = Object.fromEntries(hits.map((h) => [h.path.replace(/\\/g, '/'), h.match]))
    expect(by['R:/Roaming/Thorium']).toBe('exact')
    expect(by['R:/Roaming/EDRLab/Thorium Reader']).toBe('loose')
    expect(by['R:/Roaming/thorium-updater']).toBe('loose')
    expect(by['R:/Roaming/Microsoft']).toBeUndefined()
    expect(by['R:/Roaming/Microsoft/Thorium']).toBeUndefined()
    expect(by['R:/Roaming/Google']).toBeUndefined()
    expect(by['R:/Roaming/LinkedThorium']).toBeUndefined()
    expect(by['R:/Local/Thorium.log']).toBeUndefined()
    expect(by['R:/Roaming/EDRLab']).toBeUndefined()
  })

  it('does not match a Google app to the Google parent folder', async () => {
    const hits = await findLeftovers({ name: 'Google Chrome', publisher: 'Google LLC', bundleId: null, location: null, installed: false, others: [{ name: 'Google Drive', publisher: 'Google LLC', bundleId: null, location: null }] }, roots, 'win32', io)
    expect(hits.map((h) => h.path.replace(/\\/g, '/'))).toEqual([])
  })

  it('skips folders that belong to another installed app or are ambiguous', async () => {
    const hits = await findLeftovers(
      { name: 'Thorium', publisher: null, bundleId: null, location: null, installed: false, others: [{ name: 'Thorium Updater', publisher: null, bundleId: null, location: null }] },
      roots,
      'win32',
      io,
    )
    const paths = hits.map((h) => h.path.replace(/\\/g, '/'))
    expect(paths).toContain('R:/Roaming/Thorium')
    expect(paths).not.toContain('R:/Roaming/thorium-updater')
  })

  it('suggests a publisher folder alone only when no other app shares the publisher', async () => {
    const t2: Record<string, DirEntry[]> = { 'R:/Roaming': [{ name: 'Acme Soft', dir: true, link: false }], 'R:/Roaming/Acme Soft': [{ name: 'cache', dir: true, link: false }] }
    const io2: LeftoverIo = { readdir: async (d) => t2[d.replace(/\\/g, '/')] ?? [], exists: async () => false }
    const alone = await findLeftovers({ name: 'Widget', publisher: 'Acme Soft Inc.', bundleId: null, location: null, installed: false, others: [] }, [roots[0]!], 'win32', io2)
    expect(alone).toEqual([expect.objectContaining({ match: 'loose' })])
    const shared = await findLeftovers({ name: 'Widget', publisher: 'Acme Soft Inc.', bundleId: null, location: null, installed: false, others: [{ name: 'Gadget', publisher: 'Acme Soft', bundleId: null, location: null }] }, [roots[0]!], 'win32', io2)
    expect(shared).toEqual([])
  })

  it('matches macOS bundle ids in Preferences and Containers', async () => {
    const t3: Record<string, DirEntry[]> = {
      '/u/Library/Preferences': [
        { name: 'com.example.Tool.plist', dir: false, link: false },
        { name: 'com.example.Tool.helper.plist', dir: false, link: false },
        { name: 'com.example.Toolbox.plist', dir: false, link: false },
      ],
      '/u/Library/Containers': [{ name: 'com.example.Tool', dir: true, link: false }],
    }
    const io3: LeftoverIo = { readdir: async (d) => t3[d] ?? [], exists: async () => false }
    const hits = await findLeftovers(
      { name: 'Tool', publisher: null, bundleId: 'com.example.Tool', location: '/Applications/Tool.app', installed: false, others: [] },
      [
        { base: 'preferences', dir: '/u/Library/Preferences', files: true },
        { base: 'containers', dir: '/u/Library/Containers' },
      ],
      'darwin',
      io3,
    )
    const by = Object.fromEntries(hits.map((h) => [h.path, h.match]))
    expect(by['/u/Library/Preferences/com.example.Tool.plist']).toBe('exact')
    expect(by['/u/Library/Preferences/com.example.Tool.helper.plist']).toBe('loose')
    expect(by['/u/Library/Preferences/com.example.Toolbox.plist']).toBeUndefined()
    expect(by['/u/Library/Containers/com.example.Tool']).toBe('exact')
  })
})

describe('service', () => {
  let root: string
  beforeEach(() => {
    root = fs.mkdtempSync(path.join(os.tmpdir(), 'ccm-diskapps-'))
  })
  afterEach(() => {
    fs.rmSync(root, { recursive: true, force: true })
  })

  const app = (over: Partial<InstalledApp>): InstalledApp => ({
    id: String.raw`HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\Uninstall\Fake`,
    name: 'Fake App',
    publisher: 'Fake Co',
    version: '1.0',
    installedAt: null,
    size: null,
    sizeFrom: null,
    location: null,
    source: 'registry',
    scope: 'user',
    arch: null,
    bundleId: null,
    lastUsed: null,
    uninstall: null,
    blocked: null,
    ...over,
  })

  const make = (apps: () => InstalledApp[], extra: Partial<DiskAppsDeps> = {}) => {
    const launched: string[] = []
    const asked: string[] = []
    const svc = new DiskAppsService({
      platform: 'win32',
      home: root,
      env: { SystemRoot: String.raw`C:\Windows` },
      regFixture: null,
      macRoots: [],
      selfBundle: null,
      leftoverRoots: [{ base: 'appdata', dir: path.join(root, 'Roaming') }],
      indexSizes: async () => null,
      exists: (f) => fs.existsSync(f),
      confirm: async (o) => {
        asked.push(String(o.detail))
        return true
      },
      confirmDelete: async () => true,
      autoConfirm: false,
      launch: async (p) => {
        launched.push(p.display)
        return true
      },
      trash: async (p) => fs.rmSync(p, { recursive: true }),
      remove: async (p) => fs.rmSync(p, { recursive: true }),
      locale: () => 'vi',
      keep: () => [],
      keepTree: () => [],
      record: async () => undefined,
      readApps: async () => apps(),
      ...extra,
    })
    return { svc, launched, asked }
  }

  it('runs only the listed app, shows the command and refuses unknown ids', async () => {
    const exe = path.join(root, 'un inst.exe')
    fs.writeFileSync(exe, '')
    const a = app({ uninstall: `"${exe}" /S` })
    const { svc, launched, asked } = make(() => [a])
    expect(await svc.uninstall('nope')).toEqual({ ok: false, error: 'unknown' })
    await svc.list()
    expect(await svc.uninstall(a.id)).toEqual({ ok: true, mode: 'started' })
    expect(launched).toEqual([`"${exe}" /S`])
    expect(asked[0]).toContain(`"${exe}" /S`)
  })

  it('refuses when the registry changed after listing or the command is unsafe', async () => {
    const exe = path.join(root, 'u.exe')
    fs.writeFileSync(exe, '')
    let current = app({ uninstall: `"${exe}" /S` })
    const { svc, launched } = make(() => [current])
    await svc.list()
    current = app({ uninstall: `"${exe}" /S & del x` })
    expect(await svc.uninstall(current.id)).toEqual({ ok: false, error: 'changed' })
    await svc.list()
    expect(await svc.uninstall(current.id)).toEqual({ ok: false, error: 'unparsable' })
    expect(launched).toEqual([])
  })

  it('reports removed apps and deletes only listed leftovers through the guard', async () => {
    const left = path.join(root, 'Roaming', 'Fake App')
    const other = path.join(root, 'Roaming', 'Other')
    fs.mkdirSync(left, { recursive: true })
    fs.mkdirSync(other, { recursive: true })
    fs.writeFileSync(path.join(left, 'a.txt'), 'abc')
    const a = app({})
    let list = [a]
    const { svc } = make(() => list)
    await svc.list()
    list = []
    const res = await svc.list()
    expect(res.removed.map((r) => r.id)).toEqual([a.id])
    const scan = await svc.scanLeftovers(a.id)
    expect(scan!.installed).toBe(false)
    expect(scan!.items.map((i) => i.path)).toEqual([left])
    expect(scan!.items[0]!.size).toBe(3)
    const forged = await svc.deleteLeftovers({ id: a.id, paths: [other] })
    expect(forged.done).toEqual([])
    expect(fs.existsSync(other)).toBe(true)
    const ok = await svc.deleteLeftovers({ id: a.id, paths: [left] })
    expect(ok.done.map((d) => d.path)).toEqual([left])
    expect(fs.existsSync(left)).toBe(false)
  })
})
