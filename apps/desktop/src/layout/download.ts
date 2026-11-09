import { createHash, randomBytes } from 'node:crypto'
import { createWriteStream, existsSync, promises as fs } from 'node:fs'
import { get as httpsGet } from 'node:https'
import path from 'node:path'
import { extractZip, ZipError } from './zip'
import { LAYOUTLIB_VERSION, type BundleStatus } from './types'

export interface Artifact {
  artifact: 'layoutlib' | 'layoutlib-runtime' | 'layoutlib-resources'
  classifier?: 'win' | 'mac' | 'mac-arm'
  size: number
  sha256: string
}

// Pinned from apps/layout-daemon/README.md (Google Maven; sha1 cross-checked against Google's .sha1 files).
export const ARTIFACTS = {
  jar: { artifact: 'layoutlib', size: 60_286_750, sha256: '5dbdbd3e35abb8f8bab4c3019869642f515afd31c43d230ece0be082486c3ec8' },
  win: { artifact: 'layoutlib-runtime', classifier: 'win', size: 76_544_623, sha256: 'adadac21f5a9bfde4336d834ce0f9db83a478c65f27afd48751c470b3d708c6f' },
  mac: { artifact: 'layoutlib-runtime', classifier: 'mac', size: 80_275_609, sha256: '2b4462089c3976735cd79570e3ce35b4fe24d1c70112a8c75f422dac477ef732' },
  macArm: { artifact: 'layoutlib-runtime', classifier: 'mac-arm', size: 79_334_417, sha256: '2ca430114a7ec0c718b998ef12af915a724f7a2530dc3c382c616a2ca9076e38' },
  resources: { artifact: 'layoutlib-resources', size: 34_854_638, sha256: 'b4c4582e3680cbc8e0248223dc6ccf393bdedecd38cfccd03a33014b9238c559' },
} as const satisfies Record<string, Artifact>

interface Pinned {
  size: number
  sha256: string
}

const RENAME_RETRY_CODES = new Set(['EPERM', 'EBUSY', 'EACCES'])
const PARTIAL_STALE_MS = 10 * 60_000
const KEEP_OTHER_KITS = 1

export async function renameWithRetry(
  from: string,
  to: string,
  rename: (a: string, b: string) => Promise<void> = fs.rename,
  sleep: (ms: number) => Promise<void> = (ms) => new Promise((r) => setTimeout(r, ms)),
  tries = 5,
): Promise<void> {
  for (let i = 0; ; i++) {
    try {
      await rename(from, to)
      return
    } catch (err) {
      const code = (err as NodeJS.ErrnoException).code
      if (i >= tries - 1 || !code || !RENAME_RETRY_CODES.has(code)) throw err
      await sleep(100 * 2 ** i)
    }
  }
}

export function compareVersions(a: string, b: string): number {
  const pa = a.split(/[.-]/).map((x) => Number.parseInt(x, 10))
  const pb = b.split(/[.-]/).map((x) => Number.parseInt(x, 10))
  for (let i = 0; i < Math.max(pa.length, pb.length); i++) {
    const x = Number.isFinite(pa[i]) ? pa[i]! : 0
    const y = Number.isFinite(pb[i]) ? pb[i]! : 0
    if (x !== y) return x < y ? -1 : 1
  }
  return 0
}

export function kitsToPrune(names: string[], current: string): string[] {
  const others = names.filter((n) => n !== current && !n.includes('.partial-')).sort((x, y) => compareVersions(y, x))
  return others.slice(KEEP_OTHER_KITS).filter((n) => compareVersions(n, current) < 0)
}

export interface KitSpec {
  version: string
  size: number
  sha256: string
  url: string
}

const KIT_HOSTS = new Set(['github.com', 'objects.githubusercontent.com', 'release-assets.githubusercontent.com', 'github-releases.githubusercontent.com'])
const MAX_REDIRECTS = 5
const KIT_JAR = 'layout-daemon.jar'
const KIT_DIR = 'loose-kit'

const MAVEN = 'https://dl.google.com/android/maven2/com/android/tools/layoutlib'
const ALLOWED_HOST = 'dl.google.com'
const MAX_UNZIPPED = 600 * 1024 * 1024
const READY_MARKER = '.ccm-ready'
export const JAR_NAME = `layoutlib-${LAYOUTLIB_VERSION}.jar`

export function runtimeFor(platform: NodeJS.Platform, arch: string): Artifact | null {
  if (platform === 'win32') return ARTIFACTS.win
  if (platform === 'darwin') return arch === 'arm64' ? ARTIFACTS.macArm : ARTIFACTS.mac
  return null
}

export function isAllowedKitUrl(url: string): boolean {
  try {
    const u = new URL(url)
    return u.protocol === 'https:' && KIT_HOSTS.has(u.hostname) && !u.username && !u.password && (u.port === '' || u.port === '443')
  } catch {
    return false
  }
}

export function kitDirFor(base: string, version: string): string {
  return path.join(base, 'daemon', version)
}

export function artifactUrl(a: Artifact): string {
  const file = `${a.artifact}-${LAYOUTLIB_VERSION}${a.classifier ? `-${a.classifier}` : ''}.jar`
  return `${MAVEN}/${a.artifact}/${LAYOUTLIB_VERSION}/${file}`
}

/** Only https from Google Maven: the bytes become code on the daemon classpath. */
export function isAllowedUrl(url: string): boolean {
  try {
    const u = new URL(url)
    return u.protocol === 'https:' && u.hostname === ALLOWED_HOST && !u.username && !u.password && (u.port === '' || u.port === '443')
  } catch {
    return false
  }
}

export interface HttpBody {
  status: number
  length: number | null
  location?: string
  body: AsyncIterable<Buffer>
  abort: () => void
}

export type Fetcher = (url: string) => Promise<HttpBody>

/** No redirects are followed: Google Maven answers 200 directly and a redirect could leave the allowed host. */
export const httpsFetcher: Fetcher = (url) =>
  new Promise((resolve, reject) => {
    const req = httpsGet(url, { timeout: 30_000 }, (res) => {
      const len = Number(res.headers['content-length'])
      resolve({ status: res.statusCode ?? 0, length: Number.isFinite(len) ? len : null, location: res.headers.location, body: res, abort: () => req.destroy() })
    })
    req.on('timeout', () => req.destroy(new Error('timeout')))
    req.on('error', reject)
  })

export class DownloadError extends Error {
  constructor(readonly code: string) {
    super(code)
  }
}

export function followRedirects(base: Fetcher, allow: (url: string) => boolean, max = MAX_REDIRECTS): Fetcher {
  return async (url) => {
    let cur = url
    for (let hop = 0; hop <= max; hop++) {
      if (!allow(cur)) throw new DownloadError('host-not-allowed')
      const res = await base(cur)
      if (res.status >= 300 && res.status < 400 && res.location) {
        res.abort()
        cur = new URL(res.location, cur).toString()
        continue
      }
      return res
    }
    throw new DownloadError('too-many-redirects')
  }
}

/** Streams `a` into `file`, refusing anything that is not exactly the pinned size + sha256. */
export async function downloadArtifact(a: Artifact, file: string, fetcher: Fetcher, onBytes: (n: number) => void, url?: string): Promise<void>
export async function downloadArtifact(a: Pinned, file: string, fetcher: Fetcher, onBytes: (n: number) => void, url: string, allow: (url: string) => boolean): Promise<void>
export async function downloadArtifact(a: Pinned | Artifact, file: string, fetcher: Fetcher, onBytes: (n: number) => void, url?: string, allow: (url: string) => boolean = isAllowedUrl): Promise<void> {
  const target = url ?? ('artifact' in a ? artifactUrl(a) : '')
  return fetchPinned(a, file, fetcher, onBytes, target, allow)
}

async function fetchPinned(a: Pinned, file: string, fetcher: Fetcher, onBytes: (n: number) => void, url: string, allow: (url: string) => boolean): Promise<void> {
  if (!allow(url)) throw new DownloadError('host-not-allowed')
  const res = await fetcher(url).catch((err: unknown) => {
    throw err instanceof DownloadError ? err : new DownloadError('network')
  })
  if (res.status !== 200) {
    res.abort()
    throw new DownloadError(`http-${res.status}`)
  }
  if (res.length !== null && res.length !== a.size) {
    res.abort()
    throw new DownloadError('size-mismatch')
  }
  const hash = createHash('sha256')
  const out = createWriteStream(file)
  let got = 0
  try {
    for await (const chunk of res.body) {
      got += chunk.length
      if (got > a.size) {
        res.abort()
        throw new DownloadError('size-mismatch')
      }
      hash.update(chunk)
      if (!out.write(chunk)) await new Promise<void>((r) => out.once('drain', () => r()))
      onBytes(chunk.length)
    }
  } catch (err) {
    out.destroy()
    await fs.rm(file, { force: true })
    throw err instanceof DownloadError ? err : new DownloadError('network')
  }
  await new Promise<void>((resolve, reject) => out.end((err?: Error | null) => (err ? reject(err) : resolve())))
  if (got !== a.size) {
    await fs.rm(file, { force: true })
    throw new DownloadError('size-mismatch')
  }
  if (hash.digest('hex') !== a.sha256) {
    await fs.rm(file, { force: true })
    throw new DownloadError('sha256-mismatch')
  }
}

export interface BundleDeps {
  /** `<dataDir>/layoutlib`; the bundle lands in `<base>/<version>/`. */
  base: string
  platform: NodeJS.Platform
  arch: string
  fetcher?: Fetcher
  /** Dev: an already staged folder (apps/layout-daemon/build/layoutlib-dist) used instead of downloading. */
  overrideDir?: string | null
  kit?: KitSpec | null
  onStatus?: (status: BundleStatus) => void
}

export class LayoutlibBundle {
  private downloading: Promise<void> | null = null
  private pct = 0
  private lastError: string | null = null

  constructor(private readonly deps: BundleDeps) {}

  get dir(): string {
    const o = this.deps.overrideDir
    if (o && existsSync(path.join(o, JAR_NAME))) return o
    return path.join(this.deps.base, LAYOUTLIB_VERSION)
  }

  get kitDir(): string | null {
    const k = this.deps.kit
    return k ? kitDirFor(this.deps.base, k.version) : null
  }

  get hosts(): string {
    return [...(this.layoutlibReady() ? [] : ['dl.google.com']), ...(this.kitReady() ? [] : ['github.com (GitHub release assets)'])].join(', ') || 'dl.google.com'
  }

  /** Total compressed bytes still missing for this OS (for the consent dialog). */
  get downloadBytes(): number {
    const runtime = runtimeFor(this.deps.platform, this.deps.arch)
    const lib = this.layoutlibReady() ? 0 : ARTIFACTS.jar.size + ARTIFACTS.resources.size + (runtime?.size ?? 0)
    return lib + (this.kitReady() ? 0 : (this.deps.kit?.size ?? 0))
  }

  private layoutlibReady(): boolean {
    const dir = this.dir
    if (dir !== path.join(this.deps.base, LAYOUTLIB_VERSION)) return true
    return existsSync(path.join(dir, READY_MARKER)) && existsSync(path.join(dir, JAR_NAME))
  }

  private kitReady(): boolean {
    const dir = this.kitDir
    return !dir || (existsSync(path.join(dir, READY_MARKER)) && existsSync(path.join(dir, KIT_JAR)) && existsSync(path.join(dir, KIT_DIR, 'kit.json')))
  }

  kitPending(): boolean {
    return !this.kitReady()
  }

  isReady(): boolean {
    return this.layoutlibReady() && this.kitReady()
  }

  status(): BundleStatus {
    if (this.downloading) return { state: 'downloading', pct: this.pct }
    if (this.isReady()) return { state: 'ready', dir: this.dir }
    if (this.lastError) return { state: 'error', code: this.lastError }
    return { state: 'missing' }
  }

  download(): Promise<BundleStatus> {
    if (this.isReady()) return Promise.resolve(this.status())
    this.downloading ??= this.run().finally(() => {
      this.downloading = null
      this.deps.onStatus?.(this.status())
    })
    return this.downloading.then(() => this.status())
  }

  private emit(pct: number): void {
    const next = Math.min(100, Math.floor(pct))
    if (next === this.pct) return
    this.pct = next
    this.deps.onStatus?.({ state: 'downloading', pct: next })
  }

  private async run(): Promise<void> {
    const { base, platform, arch } = this.deps
    const needLib = !this.layoutlibReady()
    const needKit = !this.kitReady()
    const runtime = runtimeFor(platform, arch)
    if (needLib && !runtime) {
      this.lastError = 'unsupported-os'
      return
    }
    this.lastError = null
    this.pct = 0
    this.deps.onStatus?.({ state: 'downloading', pct: 0 })
    const fetcher = this.deps.fetcher ?? httpsFetcher
    const total = this.downloadBytes
    let done = 0
    const onBytes = (n: number): void => {
      done += n
      this.emit((done / total) * 85)
    }
    try {
      await fs.mkdir(base, { recursive: true })
      if (needKit) {
        await this.fetchKit(fetcher, onBytes).catch((err: unknown) => {
          throw new DownloadError(err instanceof DownloadError || err instanceof ZipError ? `daemon-${err.code}` : 'daemon-io')
        })
      }
      if (needLib && runtime) await this.fetchLayoutlib(runtime, fetcher, onBytes)
      this.emit(100)
    } catch (err) {
      this.lastError = err instanceof DownloadError || err instanceof ZipError ? err.code : 'io'
    }
  }

  private async fetchKit(fetcher: Fetcher, onBytes: (n: number) => void): Promise<void> {
    const kit = this.deps.kit
    const dest = this.kitDir
    if (!kit || !dest) return
    const kitBase = path.dirname(dest)
    await fs.mkdir(kitBase, { recursive: true })
    const stale = await fs.readdir(kitBase).catch(() => [] as string[])
    for (const n of stale) {
      if (!n.includes('.partial-')) continue
      const full = path.join(kitBase, n)
      const old = n.startsWith(`${kit.version}.partial-`) || Date.now() - ((await fs.stat(full).catch(() => null))?.mtimeMs ?? 0) > PARTIAL_STALE_MS
      if (old) await fs.rm(full, { recursive: true, force: true }).catch(() => undefined)
    }
    const tmp = path.join(kitBase, `${kit.version}.partial-${randomBytes(4).toString('hex')}`)
    try {
      await fs.mkdir(tmp, { recursive: true })
      const zipFile = path.join(tmp, 'kit.download.zip')
      await downloadArtifact(kit, zipFile, followRedirects(fetcher, isAllowedKitUrl), onBytes, kit.url, isAllowedKitUrl)
      const content = path.join(tmp, 'content')
      await extractZip(zipFile, content, { maxTotal: MAX_UNZIPPED })
      if (!existsSync(path.join(content, KIT_JAR)) || !existsSync(path.join(content, KIT_DIR, 'kit.json'))) throw new DownloadError('kit-incomplete')
      await fs.writeFile(path.join(content, READY_MARKER), `${kit.version}\n`)
      await fs.rm(dest, { recursive: true, force: true })
      await renameWithRetry(content, dest)
    } finally {
      await fs.rm(tmp, { recursive: true, force: true }).catch(() => undefined)
    }
    const names = await fs.readdir(kitBase).catch(() => [] as string[])
    for (const n of kitsToPrune(names, kit.version)) await fs.rm(path.join(kitBase, n), { recursive: true, force: true }).catch(() => undefined)
  }

  private async fetchLayoutlib(runtime: Artifact, fetcher: Fetcher, onBytes: (n: number) => void): Promise<void> {
    const { base } = this.deps
    const tmp = path.join(base, `${LAYOUTLIB_VERSION}.partial-${randomBytes(4).toString('hex')}`)
    try {
      await this.sweepPartials()
      await fs.mkdir(tmp, { recursive: true })
      const runtimeJar = path.join(tmp, 'runtime.download.jar')
      const resourcesJar = path.join(tmp, 'resources.download.jar')
      await downloadArtifact(ARTIFACTS.jar, path.join(tmp, JAR_NAME), fetcher, onBytes)
      await downloadArtifact(runtime, runtimeJar, fetcher, onBytes)
      await downloadArtifact(ARTIFACTS.resources, resourcesJar, fetcher, onBytes)
      await extractZip(runtimeJar, path.join(tmp, 'runtime'), { maxTotal: MAX_UNZIPPED })
      this.emit(92)
      await extractZip(resourcesJar, path.join(tmp, 'resources'), { maxTotal: MAX_UNZIPPED })
      await fs.rm(runtimeJar, { force: true })
      await fs.rm(resourcesJar, { force: true })
      await fs.writeFile(path.join(tmp, READY_MARKER), `${LAYOUTLIB_VERSION}\n`)
      const dest = path.join(base, LAYOUTLIB_VERSION)
      await fs.rm(dest, { recursive: true, force: true })
      await renameWithRetry(tmp, dest)
    } catch (err) {
      await fs.rm(tmp, { recursive: true, force: true }).catch(() => undefined)
      throw err
    }
  }

  private async sweepPartials(): Promise<void> {
    const names = await fs.readdir(this.deps.base).catch(() => [] as string[])
    for (const n of names) if (n.startsWith(`${LAYOUTLIB_VERSION}.partial-`)) await fs.rm(path.join(this.deps.base, n), { recursive: true, force: true }).catch(() => undefined)
  }
}
