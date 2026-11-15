import { spawn } from 'node:child_process'
import { existsSync, mkdtempSync, rmSync, statSync } from 'node:fs'
import os from 'node:os'
import path from 'node:path'

const argv = process.argv.slice(2)
const flags = new Set(argv.filter((a) => a.startsWith('--')))
const drive = (argv.find((a) => !a.startsWith('--')) ?? 'C').replace(/[:\\/]+$/, '').toUpperCase()
const level = argv.find((a) => a.startsWith('--level='))?.slice(8) ?? 'balanced'
const compare = !flags.has('--no-compare')

if (process.platform !== 'win32') {
  console.error('mft-smoke: Windows only')
  process.exit(2)
}
if (!/^[A-Z]$/.test(drive)) {
  console.error('usage: node scripts/mft-smoke.mjs [C] [--no-compare] [--level=light|balanced|max] [--bin=<orrery-core.exe>]')
  process.exit(2)
}

const repo = path.resolve(path.dirname(new URL(import.meta.url).pathname.replace(/^\/([A-Za-z]:)/, '$1')), '..')
const pick = () => {
  const given = argv.find((a) => a.startsWith('--bin='))?.slice(6) ?? process.env.CCM_CORE_BIN
  if (given) return given
  const found = ['release', 'debug'].map((p) => path.join(repo, 'apps', 'core', 'target', p, 'orrery-core.exe')).filter((f) => existsSync(f))
  found.sort((a, b) => statSync(b).mtimeMs - statSync(a).mtimeMs)
  return found[0]
}
const bin = pick()
if (!bin || !existsSync(bin)) {
  console.error('mft-smoke: orrery-core.exe not found, build it with: cd apps/core && cargo build -p core-bin --release')
  process.exit(2)
}

const data = mkdtempSync(path.join(os.tmpdir(), 'orrery-mft-smoke-'))
const env = { ...process.env, CCM_DATA_DIR: data }
delete env.CCM_MFT_FIXTURE
const child = spawn(bin, ['disk'], { env, stdio: ['pipe', 'pipe', 'inherit'], windowsHide: true })
let buf = ''
let nextId = 1
const pending = new Map()
const waiters = []
let lastProgress = 0

child.stdout.on('data', (chunk) => {
  buf += String(chunk)
  const lines = buf.split('\n')
  buf = lines.pop() ?? ''
  for (const line of lines) {
    if (!line.trim()) continue
    let m
    try {
      m = JSON.parse(line)
    } catch {
      continue
    }
    if (typeof m.id === 'number' && pending.has(m.id)) {
      const p = pending.get(m.id)
      pending.delete(m.id)
      if (m.ok) p.resolve(m.value)
      else p.reject(new Error(m.error))
    } else if (m.event === 'disk:progress') {
      const now = Date.now()
      if (now - lastProgress > 1000) {
        lastProgress = now
        const p = m.payload
        const what = p.phase === 'mft' ? `records ${p.records}/${p.total}` : `files ${p.files} dirs ${p.dirs}`
        process.stdout.write(`  ... ${(p.elapsedMs / 1000).toFixed(1)} s, ${what}\n`)
      }
    } else if (m.event === 'disk:done') {
      waiters.shift()?.(m.payload)
    }
  }
})

const call = (channel, args = {}) =>
  new Promise((resolve, reject) => {
    const id = nextId++
    pending.set(id, { resolve, reject })
    child.stdin.write(`${JSON.stringify({ id, channel, args })}\n`)
  })
const done = () => new Promise((resolve) => waiters.push(resolve))
const gb = (n) => `${(n / 1024 ** 3).toFixed(3)} GB`

async function timed(label, channel, args) {
  console.log(`\n${label}`)
  const t = Date.now()
  const wait = done()
  await call(channel, args)
  const d = await wait
  const ms = Date.now() - t
  if (!d.ok) {
    console.log(`  FAILED: ${d.error ?? (d.cancelled ? 'cancelled' : 'unknown')} after ${(ms / 1000).toFixed(1)} s`)
    return null
  }
  console.log(`  time      ${(ms / 1000).toFixed(2)} s (core ${(d.scanMs / 1000).toFixed(2)} s)`)
  console.log(`  entries   ${d.entries} (${d.files} files, ${d.dirs} dirs)`)
  console.log(`  on disk   ${gb(d.bytes)} (${d.bytes} B)`)
  console.log(`  denied    ${d.denied}  errors ${d.errors}`)
  console.log(`  index ram ${(d.indexBytes / 1024 ** 2).toFixed(1)} MB`)
  return { ms, ...d }
}

try {
  await new Promise((resolve, reject) => {
    const t = setTimeout(() => reject(new Error('core did not start')), 10_000)
    child.stdout.once('data', () => {
      clearTimeout(t)
      resolve()
    })
  })
  const vols = await call('disk.volumes')
  const vol = vols.find((v) => v.mount.toUpperCase().startsWith(`${drive}:`))
  console.log(`orrery-core ${bin}`)
  console.log(`volume      ${drive}: fs=${vol?.fs ?? '?'} kind=${vol?.kind ?? '?'} used=${vol ? gb(vol.total - vol.free) : '?'}`)
  if (vol?.fs === 'NTFS') console.log('A UAC prompt will appear for the fast scan: approve it to continue, No ends with elevation-declined.')
  const mft = await timed(`MFT fast scan of ${drive}:\\`, 'disk.mft', { drive, merge: false })
  if (mft && compare) {
    const walk = await timed(`Normal scan of ${drive}:\\ (level ${level})`, 'disk.scan', { roots: [`${drive}:\\`], level, merge: false })
    if (walk) {
      const diff = mft.bytes - walk.bytes
      console.log('\nCompare (MFT minus normal)')
      console.log(`  speed     ${(walk.ms / Math.max(mft.ms, 1)).toFixed(1)}x faster`)
      console.log(`  entries   ${mft.entries - walk.entries} (${(((mft.entries - walk.entries) / Math.max(walk.entries, 1)) * 100).toFixed(2)}%)`)
      console.log(`  on disk   ${gb(diff)} (${((diff / Math.max(walk.bytes, 1)) * 100).toFixed(2)}%)`)
      if (vol) console.log(`  volume used ${gb(vol.total - vol.free)}: MFT covers ${((mft.bytes / Math.max(vol.total - vol.free, 1)) * 100).toFixed(1)}%, normal ${((walk.bytes / Math.max(vol.total - vol.free, 1)) * 100).toFixed(1)}%`)
    }
  }
} catch (err) {
  console.error(`mft-smoke: ${err instanceof Error ? err.message : String(err)}`)
  process.exitCode = 1
} finally {
  child.stdin.end(`${JSON.stringify({ type: 'shutdown' })}\n`)
  await new Promise((r) => child.once('exit', r))
  rmSync(data, { recursive: true, force: true })
}
