import { closeSync, mkdirSync, openSync, rmSync, writeSync } from 'node:fs'
import { createRequire } from 'node:module'
import os from 'node:os'
import path from 'node:path'
import { fileURLToPath, pathToFileURL } from 'node:url'

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const require = createRequire(path.join(ROOT, 'apps', 'collector', 'package.json'))
const { UsageIndex } = await import(pathToFileURL(require.resolve('./src/usage.ts')).href)
const targetBytes = Number(process.argv.find((x) => x.startsWith('--bytes='))?.split('=')[1] ?? 4 * 1024 ** 3)
const suppliedDir = process.argv.find((x) => x.startsWith('--dir='))?.split('=').slice(1).join('=')
const base = suppliedDir ?? path.join(os.tmpdir(), `orrery-usage-corpus-${process.pid}`)
const projects = path.join(base, 'projects')
const cache = path.join(base, 'usage-index.json')
const projectsCount = 12
const filesPerProject = 16
const files = projectsCount * filesPerProject
const fileBytes = Math.ceil(targetBytes / files)
const fill = 'x'.repeat(3000)
const fixedLine = (rec) => {
  const line = JSON.stringify(rec)
  return line + ' '.repeat(Math.max(0, 4095 - line.length)) + '\n'
}
const padding = fixedLine({ type: 'user', timestamp: '2026-11-08T12:00:00.000Z', message: { role: 'user', content: fill } })
const usageTemplate = (n) => fixedLine({ type: 'assistant', timestamp: '2026-11-08T12:00:01.000Z', cwd: `D:\\work\\project-${n % projectsCount}`, message: { id: `msg-${n}`, model: 'claude-opus-4-5', usage: { input_tokens: 1200, output_tokens: 320, cache_read_input_tokens: 24000, cache_creation_input_tokens: 12000 }, content: [{ type: 'text', text: fill }] } })
const lineSize = Buffer.byteLength(padding)
const rowsPerFile = Math.floor(fileBytes / lineSize)
const started = performance.now()

rmSync(base, { recursive: true, force: true })
mkdirSync(projects, { recursive: true })
let serial = 0
for (let p = 0; p < projectsCount; p++) {
  for (let f = 0; f < filesPerProject; f++) {
    const dir = path.join(projects, `project-${p}`, `session-${f}`)
    mkdirSync(dir, { recursive: true })
    const fd = openSync(path.join(dir, `transcript-${f}.jsonl`), 'w')
    const chunk = Buffer.allocUnsafe(lineSize * 256)
    let used = 0
    for (let i = 0; i < rowsPerFile; i++) {
      const row = serial % 100 === 0 ? usageTemplate(serial) : padding
      serial++
      const b = Buffer.from(row)
      b.copy(chunk, used)
      used += b.length
      if (used + lineSize > chunk.length || i === rowsPerFile - 1) {
        writeSync(fd, chunk, 0, used)
        used = 0
      }
    }
    closeSync(fd)
  }
}
let corpusBytes = 0
const { readdir, stat: statFile } = await import('node:fs/promises')
const walk = async (dir) => {
  for (const ent of await readdir(dir, { withFileTypes: true })) {
    const full = path.join(dir, ent.name)
    if (ent.isDirectory()) await walk(full)
    else corpusBytes += (await statFile(full)).size
  }
}
await walk(projects)

const measure = async (label, cacheFile) => {
  const cpu0 = process.cpuUsage()
  const t0 = performance.now()
  let peakRss = process.memoryUsage().rss
  const sampler = setInterval(() => { peakRss = Math.max(peakRss, process.memoryUsage().rss) }, 100)
  const sink = { resetUsage() {}, updateUsage() {}, setUsageScan() {} }
  const index = new UsageIndex(projects, cacheFile, sink, { sweepMs: 0, saveDebounceMs: 0 })
  await index.start()
  while (index.scanInfo().state !== 'ready' && performance.now() - t0 < 30 * 60_000) await new Promise((r) => setTimeout(r, 25))
  await index.whenIdle()
  const wallMs = performance.now() - t0
  const cpu = process.cpuUsage(cpu0)
  clearInterval(sampler)
  await index.stop()
  return { mode: label, wallMs: Math.round(wallMs), cpuSeconds: Math.round((cpu.user + cpu.system) / 10000) / 100, peakRssMB: Math.round(peakRss / 1048576), files: index.scanInfo().filesTotal, state: index.scanInfo().state }
}

const cold = await measure('cold', cache)
const warm = await measure('warm', cache)
console.log(JSON.stringify({ corpusDir: base, corpusBytes, targetBytes, files, lineBytes: lineSize, usageRecordEvery: 100, generationWallMs: Math.round(performance.now() - started), scans: [cold, warm] }, null, 2))
