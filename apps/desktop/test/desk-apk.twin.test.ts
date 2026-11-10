import { mkdirSync, mkdtempSync, readdirSync, realpathSync, readFileSync, rmSync, utimesSync, writeFileSync } from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { afterAll, beforeAll, describe, expect, it } from 'vitest'
import { registerApkIpc } from '../src/apk/ipc'
import { aabManifest, apkManifest, dex, pb } from './apk-fixtures'
import { fakeIpc, HAVE_DESK, startTwin, type Twin } from './desk-twin'
import { makeZip } from './make-zip'

const tmp = realpathSync(mkdtempSync(path.join(os.tmpdir(), 'ccm-desk-apk-')))
const proj = path.join(tmp, 'proj')
const outside = path.join(tmp, 'outside')
const dataNode = path.join(tmp, 'data-node')
const dataRust = path.join(tmp, 'data-rust')
let picked: string | null = null

let twin: Twin
const node = fakeIpc()
const rust = fakeIpc()
const ev = {} as never

const norm = (v: unknown): unknown => JSON.parse(JSON.stringify(v).replace(/"analyzedAt":[\d.]+/g, '"analyzedAt":1'))

const writeApk = (file: string, files: { name: string; data: Buffer; store?: boolean }[], sec: number) => {
  writeFileSync(file, makeZip(files))
  utimesSync(file, sec, sec)
}

describe.skipIf(!HAVE_DESK)('apk: TS handler and orrery-core desk give the same answers', () => {
  beforeAll(async () => {
    for (const d of [proj, outside, dataNode, dataRust]) mkdirSync(d, { recursive: true })
    twin = await startTwin({ CCM_DATA_DIR: dataRust })
    const make = (ipc: ReturnType<typeof fakeIpc>, dataDir: string, withDesk: boolean) =>
      registerApkIpc({
        ipcMain: ipc.ipcMain,
        isAppFrame: () => true,
        resolveFile: async (p) => (typeof p === 'string' && p.startsWith(proj) ? p : null),
        resolveDir: async (d) => (d === proj ? proj : null),
        dialog: { showOpenDialog: async () => ({ canceled: !picked, filePaths: picked ? [picked] : [] }) } as never,
        getWindow: () => null,
        dataDir,
        desk: withDesk ? twin.router : undefined,
      })
    make(node, dataNode, false)
    make(rust, dataRust, true)
  })

  afterAll(async () => {
    await twin.host.stop()
    rmSync(tmp, { recursive: true, force: true })
  })

  const both = async (channel: string, ...args: unknown[]) => {
    const a = norm(await node.call(channel, ev, ...args))
    const b = norm(await rust.call(channel, ev, ...args))
    expect(b).toEqual(a)
    return a as Record<string, unknown>
  }

  it('analyzes an apk, remembers it and diffs the next build', async () => {
    const file = path.join(proj, 'app-release.apk')
    writeApk(
      file,
      [
        { name: 'AndroidManifest.xml', data: apkManifest() },
        { name: 'classes.dex', data: dex(100) },
        { name: 'classes2.dex', data: dex(50) },
        { name: 'resources.arsc', data: Buffer.alloc(4000, 7) },
        { name: 'res/a.png', data: Buffer.alloc(9000, 1), store: true },
        { name: 'lib/arm64-v8a/libx.so', data: Buffer.alloc(30000, 3) },
        { name: 'assets/a.txt', data: Buffer.from('hello') },
        { name: 'META-INF/CERT.RSA', data: Buffer.from('cert') },
        { name: 'kotlin/x.kotlin_builtins', data: Buffer.from('k') },
        { name: 'dir/', data: Buffer.alloc(0), store: true },
      ],
      1_700_000_000,
    )
    const first = await both('ccm:apk-analyze', { path: file, root: proj })
    expect(first.ok).toBe(true)
    expect(first.previous).toBeNull()
    writeApk(file, [{ name: 'AndroidManifest.xml', data: apkManifest() }, { name: 'classes.dex', data: dex(120) }, { name: 'res/a.png', data: Buffer.alloc(5000, 2), store: true }, { name: 'assets/new.bin', data: Buffer.alloc(2000, 9) }], 1_700_000_100)
    const second = await both('ccm:apk-analyze', { path: file, root: proj })
    expect(second.previous).not.toBeNull()
    expect((second.diff as { addedCount: number }).addedCount).toBe(1)
    await both('ccm:apk-analyze', { path: file, root: proj })
    await both('ccm:apk-analyze', { path: file })
    const files = readdirSync(path.join(dataRust, 'apk-history'))
    expect(files).toEqual(readdirSync(path.join(dataNode, 'apk-history')))
    for (const f of files) expect(norm(JSON.parse(readFileSync(path.join(dataRust, 'apk-history', f), 'utf8')))).toEqual(norm(JSON.parse(readFileSync(path.join(dataNode, 'apk-history', f), 'utf8'))))
  })

  it('reads a bundle and compares two files', async () => {
    const aab = path.join(proj, 'app.aab')
    writeApk(aab, [{ name: 'BundleConfig.pb', data: Buffer.from('x') }, { name: 'base/manifest/AndroidManifest.xml', data: aabManifest() }, { name: 'base/dex/classes.dex', data: dex(7) }, { name: 'camera/dex/classes.dex', data: dex(3) }, { name: 'base/resources.pb', data: pb(1, 'r') }], 1_700_000_200)
    const one = await both('ccm:apk-analyze', { path: aab, root: proj })
    expect((one.report as { kind: string }).kind).toBe('aab')
    const apk = path.join(proj, 'app-release.apk')
    const cmp = await both('ccm:apk-compare', { path: apk, other: aab })
    expect(cmp.ok).toBe(true)
    await both('ccm:apk-compare', { path: apk, other: path.join(proj, 'gone.apk') })
    await both('ccm:apk-compare', { path: apk })
  })

  it('refuses outside paths, bad names and non-objects the same way', async () => {
    const out = path.join(outside, 'o.apk')
    writeApk(out, [{ name: 'classes.dex', data: dex(1) }], 1_700_000_300)
    expect(await both('ccm:apk-analyze', { path: out })).toEqual({ ok: false, error: 'denied' })
    await both('ccm:apk-analyze', { path: path.join(proj, 'a.txt') })
    await both('ccm:apk-analyze', 'x')
    await both('ccm:apk-analyze', null)
    await both('ccm:apk-analyze', { path: 'rel.apk' })
    await both('ccm:apk-analyze', { path: path.join(proj, 'gone.apk') })
    writeFileSync(path.join(proj, 'junk.apk'), 'not a zip at all')
    expect(await both('ccm:apk-analyze', { path: path.join(proj, 'junk.apk') })).toEqual({ ok: false, error: 'not-zip' })
  })

  it('a picked file is readable outside every root', async () => {
    const out = path.join(outside, 'o.apk')
    picked = null
    expect(await both('ccm:apk-pick', proj)).toBeNull()
    picked = out
    expect(await both('ccm:apk-pick', proj)).toBe(out)
    const r = await both('ccm:apk-analyze', { path: out })
    expect(r.ok).toBe(true)
    picked = path.join(outside, 'x.txt')
    expect(await both('ccm:apk-pick', null)).toBeNull()
  })

  it('stayed on the rust path', () => {
    expect(twin.logs).toEqual([])
    expect(twin.fellBack).toEqual([])
  })
})
