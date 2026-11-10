import { mkdirSync, mkdtempSync, readdirSync, readFileSync, realpathSync, rmSync, writeFileSync, existsSync } from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { afterAll, describe, expect, it } from 'vitest'
import { LayoutlibBundle, compareVersions, followRedirects, kitsToPrune, renameWithRetry, isAllowedKitUrl, JAR_NAME, kitDirFor, type Fetcher, type HttpBody, type KitSpec } from '../src/layout/download'
import { kitSpecFor } from '../src/layout/kit'
import { extractZip } from '../src/layout/zip'
import { buildZip, isPinned, KIT_FILE, kitEmbed, kitNames, makeKitPin, readKitPin, sha256Of } from '../daemon-kit.mjs'
import { parseArgs, releaseCommand } from '../../../scripts/kit-release.mjs'
import { installStatus, uninstallBundle } from '../src/layout/install'

const root = realpathSync.native(mkdtempSync(path.join(os.tmpdir(), 'ccm-kit-')))
afterAll(() => rmSync(root, { recursive: true, force: true }))

let n = 0
const fresh = (): { base: string; override: string } => {
  const dir = path.join(root, `case-${n++}`)
  const override = path.join(dir, 'll')
  mkdirSync(override, { recursive: true })
  writeFileSync(path.join(override, JAR_NAME), 'x')
  return { base: path.join(dir, 'layoutlib'), override }
}

const kitZip = (version: string, extra: { name: string; data: Buffer }[] = []): Buffer =>
  buildZip([
    { name: 'layout-daemon.jar', data: Buffer.from(`jar-${version}`) },
    { name: 'loose-kit/kit.json', data: Buffer.from('{}') },
    { name: 'loose-kit/classes/a.jar', data: Buffer.alloc(4000, 7) },
    ...extra,
  ])

const specOf = (version: string, zip: Buffer): KitSpec => kitSpecFor(version, zip.length, sha256Of(zip))

function body(buf: Buffer, status = 200, location?: string): HttpBody {
  return {
    status,
    length: buf.length,
    location,
    abort: () => undefined,
    body: (async function* () {
      for (let i = 0; i < buf.length; i += 1000) yield buf.subarray(i, i + 1000)
    })(),
  }
}

const serving = (buf: Buffer): Fetcher & { calls: string[] } => {
  const calls: string[] = []
  const f = (async (url: string) => {
    calls.push(url)
    return body(buf)
  }) as Fetcher & { calls: string[] }
  f.calls = calls
  return f
}

const bundleOf = (c: { base: string; override: string }, kit: KitSpec | null, fetcher: Fetcher): LayoutlibBundle =>
  new LayoutlibBundle({ base: c.base, platform: 'win32', arch: 'x64', fetcher, overrideDir: c.override, kit })

describe('daemon kit zip', () => {
  it('buildZip output extracts with the app extractor and hashes deterministically', async () => {
    const zip = kitZip('1.0.0')
    expect(sha256Of(kitZip('1.0.0'))).toBe(sha256Of(zip))
    const file = path.join(root, 'k.zip')
    writeFileSync(file, zip)
    const out = path.join(root, 'k-out')
    const r = await extractZip(file, out, { maxTotal: 1_000_000 })
    expect(r.files).toBe(3)
    expect(readFileSync(path.join(out, 'layout-daemon.jar'), 'utf8')).toBe('jar-1.0.0')
    expect(readFileSync(path.join(out, 'loose-kit', 'classes', 'a.jar')).length).toBe(4000)
  })

  it('release url is pinned to the GitHub release of the kit version', () => {
    const k = kitSpecFor('1', 10, 'ab')
    expect(k.url).toBe('https://github.com/JLSoft-Together/Orrery/releases/download/layout-daemon-v1/layout-daemon-1.zip')
    expect(isAllowedKitUrl(k.url)).toBe(true)
    expect(isAllowedKitUrl('https://objects.githubusercontent.com/x')).toBe(true)
    expect(isAllowedKitUrl('http://github.com/x')).toBe(false)
    expect(isAllowedKitUrl('https://github.com.evil.io/x')).toBe(false)
    expect(isAllowedKitUrl('https://dl.google.com/x')).toBe(false)
  })
})

describe('followRedirects', () => {
  it('follows allowed hops and refuses a hop to another host', async () => {
    const seen: string[] = []
    const ok: Fetcher = async (url) => {
      seen.push(url)
      return url.startsWith('https://github.com') ? body(Buffer.alloc(0), 302, 'https://objects.githubusercontent.com/a') : body(Buffer.from('z'))
    }
    const res = await followRedirects(ok, isAllowedKitUrl)('https://github.com/x')
    expect(res.status).toBe(200)
    expect(seen).toEqual(['https://github.com/x', 'https://objects.githubusercontent.com/a'])
    const bad: Fetcher = async () => body(Buffer.alloc(0), 302, 'https://evil.example/a')
    await expect(followRedirects(bad, isAllowedKitUrl)('https://github.com/x')).rejects.toMatchObject({ code: 'host-not-allowed' })
    const loop: Fetcher = async () => body(Buffer.alloc(0), 302, 'https://github.com/x')
    await expect(followRedirects(loop, isAllowedKitUrl)('https://github.com/x')).rejects.toMatchObject({ code: 'too-many-redirects' })
  })
})

describe('LayoutlibBundle daemon kit', () => {
  it('without a kit spec (dev) nothing is downloaded and the bundle is ready', async () => {
    const c = fresh()
    const f = serving(Buffer.alloc(0))
    const b = bundleOf(c, null, f)
    expect(b.isReady()).toBe(true)
    expect(b.kitPending()).toBe(false)
    expect((await b.download()).state).toBe('ready')
    expect(f.calls).toHaveLength(0)
  })

  it('downloads, verifies, extracts atomically and reports ready', async () => {
    const c = fresh()
    const zip = kitZip('3.5.0')
    const kit = specOf('3.5.0', zip)
    const f = serving(zip)
    const progress: number[] = []
    const b = new LayoutlibBundle({
      base: c.base,
      platform: 'win32',
      arch: 'x64',
      fetcher: f,
      overrideDir: c.override,
      kit,
      onStatus: (s) => {
        if (s.state === 'downloading') progress.push(s.pct)
      },
    })
    expect(b.isReady()).toBe(false)
    expect(b.kitPending()).toBe(true)
    expect(b.downloadBytes).toBe(zip.length)
    expect(b.hosts).toBe('github.com (GitHub release assets)')
    expect(b.status()).toEqual({ state: 'missing' })
    const st = await b.download()
    expect(st.state).toBe('ready')
    expect(f.calls).toEqual([kit.url])
    const dir = kitDirFor(c.base, '3.5.0')
    expect(readFileSync(path.join(dir, 'layout-daemon.jar'), 'utf8')).toBe('jar-3.5.0')
    expect(existsSync(path.join(dir, 'loose-kit', 'kit.json'))).toBe(true)
    expect(readdirSync(path.join(c.base, 'daemon'))).toEqual(['3.5.0'])
    expect(progress[0]).toBe(0)
    expect(progress.at(-1)).toBe(100)
    expect(b.kitPending()).toBe(false)
  })

  it('wrong checksum fails with a clear code, leaves no partial files and can retry', async () => {
    const c = fresh()
    const zip = kitZip('3.5.0')
    const kit = specOf('3.5.0', zip)
    const tampered = Buffer.from(zip)
    tampered[40] ^= 0xff
    const b = bundleOf(c, kit, serving(tampered))
    const st = await b.download()
    expect(st).toEqual({ state: 'error', code: 'daemon-sha256-mismatch' })
    expect(b.isReady()).toBe(false)
    expect(readdirSync(path.join(c.base, 'daemon'))).toEqual([])
    const retry = bundleOf(c, kit, serving(zip))
    expect((await retry.download()).state).toBe('ready')
  })

  it('network failure and http errors are reported, not thrown', async () => {
    const c = fresh()
    const kit = specOf('3.5.0', kitZip('3.5.0'))
    const down: Fetcher = async () => {
      throw new Error('ECONNRESET')
    }
    expect(await bundleOf(c, kit, down).download()).toEqual({ state: 'error', code: 'daemon-network' })
    const missing: Fetcher = async () => body(Buffer.from('nope'), 404)
    expect(await bundleOf(c, kit, missing).download()).toEqual({ state: 'error', code: 'daemon-http-404' })
    const cut: Fetcher = async () => ({
      status: 200,
      length: kit.size,
      abort: () => undefined,
      body: (async function* () {
        yield Buffer.alloc(100)
        throw new Error('socket hang up')
      })(),
    })
    expect(await bundleOf(c, kit, cut).download()).toEqual({ state: 'error', code: 'daemon-network' })
    expect(readdirSync(path.join(c.base, 'daemon'))).toEqual([])
  })

  it('an archive without the jar or kit.json is refused', async () => {
    const c = fresh()
    const zip = buildZip([{ name: 'readme.txt', data: Buffer.from('x') }])
    const st = await bundleOf(c, specOf('3.5.0', zip), serving(zip)).download()
    expect(st).toEqual({ state: 'error', code: 'daemon-kit-incomplete' })
    expect(readdirSync(path.join(c.base, 'daemon'))).toEqual([])
  })

  it('a zip-slip entry is refused', async () => {
    const c = fresh()
    const zip = kitZip('3.5.0', [{ name: '../evil.txt', data: Buffer.from('x') }])
    const st = await bundleOf(c, specOf('3.5.0', zip), serving(zip)).download()
    expect(st).toEqual({ state: 'error', code: 'daemon-zip-unsafe-path' })
    expect(existsSync(path.join(c.base, 'daemon', 'evil.txt'))).toBe(false)
  })

  it('does not download again when this version is already there', async () => {
    const c = fresh()
    const zip = kitZip('3.5.0')
    const kit = specOf('3.5.0', zip)
    await bundleOf(c, kit, serving(zip)).download()
    const f = serving(zip)
    const b = bundleOf(c, kit, f)
    expect(b.isReady()).toBe(true)
    expect(b.downloadBytes).toBe(0)
    expect((await b.download()).state).toBe('ready')
    expect(f.calls).toHaveLength(0)
  })

  it('prunes every other kit and only sweeps its own partials', async () => {
    const c = fresh()
    const daemon = path.join(c.base, 'daemon')
    for (const v of ['3.2.0', '3.9.0']) mkdirSync(path.join(daemon, v), { recursive: true })
    mkdirSync(path.join(daemon, '3.5.0.partial-dead'), { recursive: true })
    mkdirSync(path.join(daemon, '3.9.0.partial-live'), { recursive: true })
    const zip = kitZip('3.5.0')
    const b = bundleOf(c, specOf('3.5.0', zip), serving(zip))
    expect(b.isReady()).toBe(false)
    expect((await b.download()).state).toBe('ready')
    expect(readdirSync(daemon).sort()).toEqual(['3.5.0', '3.9.0.partial-live'])
  })

  it('kitsToPrune lists every finished kit but the current one', () => {
    expect(kitsToPrune(['3.1.0', '1', '3.5.0', '3.5.0.partial-x'], '3.5.0')).toEqual(['3.1.0', '1'])
    expect(kitsToPrune(['1'], '1')).toEqual([])
    expect(compareVersions('3.10.0', '3.9.0')).toBe(1)
    expect(compareVersions('3.5.0', '3.5.0')).toBe(0)
  })

  it('retries a rename on EPERM / EBUSY / EACCES with backoff and gives up on other errors', async () => {
    const delays: number[] = []
    let n = 0
    const flaky = async (): Promise<void> => {
      if (++n < 3) throw Object.assign(new Error('busy'), { code: 'EBUSY' })
    }
    await renameWithRetry('a', 'b', flaky, async (ms) => void delays.push(ms))
    expect(n).toBe(3)
    expect(delays).toEqual([100, 200])
    const always = async (): Promise<void> => {
      throw Object.assign(new Error('perm'), { code: 'EPERM' })
    }
    await expect(renameWithRetry('a', 'b', always, async () => undefined, 4)).rejects.toMatchObject({ code: 'EPERM' })
    let m = 0
    const enoent = async (): Promise<void> => {
      m++
      throw Object.assign(new Error('x'), { code: 'ENOENT' })
    }
    await expect(renameWithRetry('a', 'b', enoent, async () => undefined)).rejects.toMatchObject({ code: 'ENOENT' })
    expect(m).toBe(1)
  })

  it('two enables at once share one download', async () => {
    const c = fresh()
    const zip = kitZip('3.5.0')
    const f = serving(zip)
    const b = bundleOf(c, specOf('3.5.0', zip), f)
    const [a1, a2] = await Promise.all([b.download(), b.download()])
    expect(a1.state).toBe('ready')
    expect(a2.state).toBe('ready')
    expect(f.calls).toHaveLength(1)
  })

  it('a directory without the ready marker counts as missing', async () => {
    const c = fresh()
    const zip = kitZip('3.5.0')
    const dir = kitDirFor(c.base, '3.5.0')
    mkdirSync(path.join(dir, 'loose-kit'), { recursive: true })
    writeFileSync(path.join(dir, 'layout-daemon.jar'), 'half')
    writeFileSync(path.join(dir, 'loose-kit', 'kit.json'), '{}')
    const f = serving(zip)
    const b = bundleOf(c, specOf('3.5.0', zip), f)
    expect(b.isReady()).toBe(false)
    expect((await b.download()).state).toBe('ready')
    expect(readFileSync(path.join(dir, 'layout-daemon.jar'), 'utf8')).toBe('jar-3.5.0')
  })

  it('an unwritable data folder ends in an error state', async () => {
    const c = fresh()
    const zip = kitZip('3.5.0')
    mkdirSync(path.dirname(c.base), { recursive: true })
    writeFileSync(c.base, 'a file where the folder should be')
    const st = await bundleOf(c, specOf('3.5.0', zip), serving(zip)).download()
    expect(st.state).toBe('error')
  })
})

describe('layout ipc with a pending daemon kit', () => {
  it('reports the jar as available, asks consent with the pending hosts, then renders after download', async () => {
    const handlers = new Map<string, (event: unknown, ...args: unknown[]) => unknown>()
    const ipcMain = { handle: (ch: string, fn: (event: unknown, ...args: unknown[]) => unknown) => handlers.set(ch, fn), removeHandler: (ch: string) => handlers.delete(ch) }
    const c = fresh()
    const zip = kitZip('3.5.0')
    const f = serving(zip)
    const asked: { bytes: number; host: string }[] = []
    const { registerLayoutIpc } = await import('../src/layout/ipc')
    const ipc = registerLayoutIpc({
      ipcMain: ipcMain as never,
      isAppFrame: () => true,
      resolveDir: async (p) => (typeof p === 'string' ? p : null),
      dataDir: path.dirname(c.base),
      getJdk: () => null,
      confirm: async (_e, info) => {
        asked.push(info)
        return true
      },
      server: { urlFor: async () => null },
      daemonJar: path.join(kitDirFor(c.base, '3.5.0'), 'layout-daemon.jar'),
      spawn: () => {
        throw new Error('no spawn')
      },
      kill: () => undefined,
      layoutlibOverride: c.override,
      kit: specOf('3.5.0', zip),
      bundle: bundleOf(c, specOf('3.5.0', zip), f),
      findJava: async () => ({ ok: false, code: 'jdk-missing' }),
    })
    const status = (await handlers.get('ccm:layout-status')!({ sender: { id: 1 } })) as { jar: boolean; bundle: { state: string } }
    expect(status.jar).toBe(true)
    expect(status.bundle.state).toBe('missing')
    const res = (await handlers.get('ccm:layout-download')!({ sender: { id: 1 } })) as { state: string }
    expect(res.state).toBe('ready')
    expect(asked).toEqual([{ bytes: zip.length, host: 'github.com (GitHub release assets)' }])
    const after = (await handlers.get('ccm:layout-status')!({ sender: { id: 1 } })) as { jar: boolean; bundle: { state: string } }
    expect(after.jar).toBe(true)
    expect(after.bundle.state).toBe('ready')
    ipc.dispose()
  })
})

describe('kit pin', () => {
  const sha = 'ab'.repeat(32)

  it('names the release after the kit version, independent of the app version', () => {
    expect(kitNames('1')).toEqual({ tag: 'layout-daemon-v1', asset: 'layout-daemon-1.zip', url: 'https://github.com/JLSoft-Together/Orrery/releases/download/layout-daemon-v1/layout-daemon-1.zip' })
    expect(makeKitPin(2, 10, sha)).toMatchObject({ version: '2', tag: 'layout-daemon-v2', asset: 'layout-daemon-2.zip', size: 10, sha256: sha })
  })

  it('a placeholder pin embeds nothing, a real one embeds version, size, sha and url', () => {
    expect(isPinned(JSON.parse(readFileSync(KIT_FILE, 'utf8')))).toBe(isPinned(readKitPin()))
    expect(kitEmbed({ version: '1', size: 0, sha256: 'PLACEHOLDER' })).toBeNull()
    expect(kitEmbed({ version: '1', size: 5, sha256: 'xyz', url: 'u' })).toBeNull()
    expect(kitEmbed(makeKitPin(1, 5, sha))).toEqual({ version: '1', size: 5, sha256: sha, url: kitNames('1').url })
  })

  it('kit-release parses its flags and prints the publish command', () => {
    expect(parseArgs([], '1')).toMatchObject({ version: '1', skipGradle: false, zip: null })
    expect(parseArgs(['--version', '3', '--skip-gradle', '--zip', 'a.zip'], '1')).toMatchObject({ version: '3', skipGradle: true, zip: 'a.zip' })
    expect(() => parseArgs(['--version', '0'], '1')).toThrow()
    expect(() => parseArgs(['--nope'], '1')).toThrow()
    expect(releaseCommand('2', '/tmp/layout-daemon-2.zip')).toContain('gh release create layout-daemon-v2 "/tmp/layout-daemon-2.zip"')
  })
})

describe('install status and uninstall', () => {
  it('reports not installed, then installed with its size, then removes everything under the layoutlib folder', async () => {
    const c = fresh()
    const zip = kitZip('1')
    const stops: number[] = []
    const b = bundleOf(c, specOf('1', zip), serving(zip))
    const before = await installStatus(b)
    expect(before).toMatchObject({ available: true, installed: false, version: null, pinnedVersion: '1', downloadBytes: zip.length, sizeBytes: 0 })
    expect((await b.download()).state).toBe('ready')
    const after = await installStatus(b)
    expect(after).toMatchObject({ installed: true, version: '1', downloadBytes: 0 })
    expect(after.sizeBytes).toBeGreaterThan(4000)
    mkdirSync(path.join(c.base, 'cache', 'snap'), { recursive: true })
    const res = await uninstallBundle(b, async () => void stops.push(1))
    expect(stops).toEqual([1])
    expect(res.ok).toBe(true)
    expect(res.status).toMatchObject({ installed: false, version: null, sizeBytes: 0 })
    expect(existsSync(c.base)).toBe(false)
    expect(b.status()).toEqual({ state: 'missing' })
  })

  it('an old kit installed under the app version is not the pinned kit: not installed, still listed and removable', async () => {
    const c = fresh()
    const old = kitDirFor(c.base, '3.5.0')
    mkdirSync(path.join(old, 'loose-kit'), { recursive: true })
    writeFileSync(path.join(old, 'layout-daemon.jar'), 'j')
    writeFileSync(path.join(old, 'loose-kit', 'kit.json'), '{}')
    writeFileSync(path.join(old, '.ccm-ready'), '3.5.0\n')
    const zip = kitZip('1')
    const b = bundleOf(c, specOf('1', zip), serving(zip))
    const st = await installStatus(b)
    expect(st).toMatchObject({ installed: false, version: '3.5.0', pinnedVersion: '1' })
    expect(st.sizeBytes).toBeGreaterThan(0)
    expect((await b.download()).state).toBe('ready')
    expect(readdirSync(path.join(c.base, 'daemon'))).toEqual(['1'])
  })

  it('without a pinned kit and without a dev bundle nothing is available', async () => {
    const dir = path.join(root, 'nokit')
    mkdirSync(dir, { recursive: true })
    const b = new LayoutlibBundle({ base: path.join(dir, 'layoutlib'), platform: 'win32', arch: 'x64', fetcher: serving(Buffer.alloc(0)), overrideDir: null, kit: null })
    expect(await installStatus(b)).toMatchObject({ available: false, installed: false, pinnedVersion: null })
  })

  it('refuses to uninstall while a download runs', async () => {
    const c = fresh()
    const zip = kitZip('1')
    let release: () => void = () => undefined
    const gate = new Promise<void>((r) => (release = r))
    const slow: Fetcher = async () => {
      await gate
      return body(zip)
    }
    const b = bundleOf(c, specOf('1', zip), slow)
    const running = b.download()
    const res = await uninstallBundle(b, async () => undefined)
    expect(res).toMatchObject({ ok: false, code: 'busy' })
    release()
    expect((await running).state).toBe('ready')
  })

  it('the ipc channels answer the status and the uninstall', async () => {
    const handlers = new Map<string, (event: unknown, ...args: unknown[]) => unknown>()
    const ipcMain = { handle: (ch: string, fn: (event: unknown, ...args: unknown[]) => unknown) => handlers.set(ch, fn), removeHandler: (ch: string) => handlers.delete(ch) }
    const c = fresh()
    const zip = kitZip('1')
    const { registerLayoutIpc } = await import('../src/layout/ipc')
    const ipc = registerLayoutIpc({
      ipcMain: ipcMain as never,
      isAppFrame: (e) => (e as { ok?: boolean }).ok !== false,
      resolveDir: async () => null,
      dataDir: path.dirname(c.base),
      getJdk: () => null,
      confirm: async () => true,
      server: { urlFor: async () => null },
      daemonJar: path.join(kitDirFor(c.base, '1'), 'layout-daemon.jar'),
      spawn: () => {
        throw new Error('no spawn')
      },
      kill: () => undefined,
      layoutlibOverride: c.override,
      kit: specOf('1', zip),
      bundle: bundleOf(c, specOf('1', zip), serving(zip)),
      findJava: async () => ({ ok: false, code: 'jdk-missing' }),
    })
    const call = (ch: string, ok = true) => handlers.get(ch)!({ sender: { id: 1 }, ok })
    expect(await call('ccm:layout-install-status', false)).toBeNull()
    expect(await call('ccm:layout-uninstall', false)).toBeNull()
    expect(await call('ccm:layout-install-status')).toMatchObject({ available: true, installed: false })
    expect(await call('ccm:layout-download')).toMatchObject({ state: 'ready' })
    expect(await call('ccm:layout-install-status')).toMatchObject({ installed: true, version: '1' })
    expect(await call('ccm:layout-uninstall')).toMatchObject({ ok: true, status: { installed: false } })
    ipc.dispose()
  })
})
