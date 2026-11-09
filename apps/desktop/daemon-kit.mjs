import { createHash } from 'node:crypto'
import { mkdirSync, readdirSync, readFileSync, statSync, writeFileSync } from 'node:fs'
import path from 'node:path'
import { crc32, deflateRawSync } from 'node:zlib'

const DOS_TIME = 0
const DOS_DATE = (1980 - 1980) * 512 + 1 * 32 + 1

export function listFiles(dir, prefix = '') {
  const out = []
  for (const name of readdirSync(dir).sort()) {
    const full = path.join(dir, name)
    if (statSync(full).isDirectory()) out.push(...listFiles(full, `${prefix}${name}/`))
    else out.push({ name: `${prefix}${name}`, file: full })
  }
  return out
}

export function buildZip(files) {
  const locals = []
  const central = []
  let offset = 0
  for (const f of files) {
    const data = f.data ?? readFileSync(f.file)
    const name = Buffer.from(f.name, 'utf8')
    const packed = deflateRawSync(data, { level: 9 })
    const store = packed.length >= data.length
    const body = store ? data : packed
    const method = store ? 0 : 8
    const crc = crc32(data)
    const lh = Buffer.alloc(30)
    lh.writeUInt32LE(0x04034b50, 0)
    lh.writeUInt16LE(20, 4)
    lh.writeUInt16LE(0x0800, 6)
    lh.writeUInt16LE(method, 8)
    lh.writeUInt16LE(DOS_TIME, 10)
    lh.writeUInt16LE(DOS_DATE, 12)
    lh.writeUInt32LE(crc, 14)
    lh.writeUInt32LE(body.length, 18)
    lh.writeUInt32LE(data.length, 22)
    lh.writeUInt16LE(name.length, 26)
    const ch = Buffer.alloc(46)
    ch.writeUInt32LE(0x02014b50, 0)
    ch.writeUInt16LE(20, 4)
    ch.writeUInt16LE(20, 6)
    ch.writeUInt16LE(0x0800, 8)
    ch.writeUInt16LE(method, 10)
    ch.writeUInt16LE(DOS_TIME, 12)
    ch.writeUInt16LE(DOS_DATE, 14)
    ch.writeUInt32LE(crc, 16)
    ch.writeUInt32LE(body.length, 20)
    ch.writeUInt32LE(data.length, 24)
    ch.writeUInt16LE(name.length, 28)
    ch.writeUInt32LE(offset, 42)
    locals.push(lh, name, body)
    central.push(ch, name)
    offset += 30 + name.length + body.length
  }
  const cd = Buffer.concat(central)
  const eocd = Buffer.alloc(22)
  eocd.writeUInt32LE(0x06054b50, 0)
  eocd.writeUInt16LE(files.length, 8)
  eocd.writeUInt16LE(files.length, 10)
  eocd.writeUInt32LE(cd.length, 12)
  eocd.writeUInt32LE(offset, 16)
  return Buffer.concat([...locals, cd, eocd])
}

export function sha256Of(buf) {
  return createHash('sha256').update(buf).digest('hex')
}

export function packDaemonKit({ jar, kitDir, out }) {
  const files = [{ name: 'layout-daemon.jar', file: jar }, ...listFiles(kitDir, 'loose-kit/')].sort((a, b) => (a.name < b.name ? -1 : a.name > b.name ? 1 : 0))
  const zip = buildZip(files)
  mkdirSync(path.dirname(out), { recursive: true })
  writeFileSync(out, zip)
  return { size: zip.length, sha256: sha256Of(zip) }
}

export function describeZip(file) {
  const buf = readFileSync(file)
  return { size: buf.length, sha256: sha256Of(buf) }
}

export const RELEASE_BASE = 'https://github.com/JLSoft-Together/Orrery/releases/download'

export function releaseZipUrl(version) {
  return `${RELEASE_BASE}/v${version}/layout-daemon-${version}.zip`
}

export async function fetchReleaseZip(version, fetchImpl = fetch) {
  try {
    const res = await fetchImpl(releaseZipUrl(version), { signal: AbortSignal.timeout(30_000) })
    if (!res.ok) return null
    return Buffer.from(await res.arrayBuffer())
  } catch {
    return null
  }
}

export function chooseKitSource({ ci, reuseFlag, zipExists, release }) {
  if (zipExists && (ci || reuseFlag)) return 'reuse'
  if (!ci && release) return 'release'
  return 'local'
}
